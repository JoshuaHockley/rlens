#![feature(associated_type_defaults)]

mod command;
mod command_types;
mod gallery;
mod geometry;
mod gfx;
mod hooks;
mod image;
mod image_loader;
mod image_transform;
mod image_view;
mod input;
mod keybinds;
mod load_request;
mod lua;
mod program;
mod rlens;
mod status_bar;
mod util;
mod window;

use lua::ConfigFlag;
use program::{rlens, Settings};
use util::{touch_dir, PrintErr};

use clap::Parser;
use directories::ProjectDirs;
use serde::Deserialize;
use std::borrow::Cow;
use std::convert::Infallible;
use std::env::var_os;
use std::fs;
use std::io::stdin;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn main() {
    main_().print_err().ok();
}

/// Command line arguments
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Paths to image files
    #[clap(value_name = "PATH")]
    paths: Vec<PathBuf>,
    /// The image to start at ([1..])
    #[clap(long = "start-at", short = 's', value_name = "INDEX")]
    initial_image: Option<usize>,
    /// Configuration flag passed into lua
    #[clap(long = "flag", short, value_name = "NAME:VALUE")]
    flags: Vec<ConfigFlag>,
    /// Path to the configuration directory
    #[clap(long, short, value_name = "DIR")]
    config_dir: Option<PathBuf>,
    /// Path to the thumbnail directory
    #[clap(long, short, value_name = "DIR")]
    thumbnail_dir: Option<PathBuf>,
}

/// Configuration file contents
#[derive(Deserialize, Default, Debug)]
struct Config {
    thumbnail_dir: Option<PathBuf>,
    thumbnail_size: Option<u32>,
    font: Option<FontConfig>,
}

#[derive(Deserialize, Default, Debug)]
struct FontConfig {
    path: Option<PathBuf>,
    size: Option<f32>,
}

fn main_() -> Result<(), String> {
    // Parse command line args
    let args = Args::parse();

    // Build the path list
    let paths = if !args.paths.is_empty() {
        // We have been given paths as command line arguments
        args.paths
    } else if !atty::is(atty::Stream::Stdin) {
        // We have data to read from stdin
        stdin()
            .lines()
            .map(|s| s.map(PathBuf::from))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Error reading from stdin: {}", e))?
    } else {
        // We have no paths
        vec![]
    };

    if paths.is_empty() {
        return Err("Error: No paths were provided".to_string());
    }

    // Initial image
    let initial_index = args
        .initial_image
        .map(|i| {
            if i < 1 || i > paths.len() {
                Err(format!(
                    "Error: Index `{}` is out of range (max: {})",
                    i,
                    paths.len()
                ))
            } else {
                Ok(i - 1)
            }
        })
        .transpose()?
        // Default to the first image
        .unwrap_or(0);

    // Build settings from the args, config file, and defaults
    const PROJECT_NAME: &str = "rlens";
    let dirs = ProjectDirs::from_path(PathBuf::from(PROJECT_NAME))
        .ok_or_else(|| "Failed to determine the system's home directory".to_string())?;

    // Config directory: Determined by args, then an environment variable, then a system standard
    const CONFIG_DIR_ENV_VAR: &str = "RLENS_CONFIG_DIR";
    let config_dir = args
        .config_dir
        .or_else(|| var_os(CONFIG_DIR_ENV_VAR).map(PathBuf::from))
        .unwrap_or_else(|| dirs.config_dir().to_path_buf());
    touch_dir(&config_dir)?;

    // The config file: Determined by `config_dir`
    const CONFIG_FILENAME: &str = "config.toml";
    let config_file_path = &{
        let mut path = config_dir.clone();
        path.push(CONFIG_FILENAME);
        path
    };
    let config = get_config(config_file_path)?;

    // The rc file: Determined by `config_dir`
    const RC_FILENAME: &str = "rc.lua";
    let rc_path = {
        let mut p = config_dir.clone();
        p.push(RC_FILENAME);
        p
    };

    // Config flags: Determined by args
    let config_flags = args.flags;

    // Thumbnail directory: Determined by args, then the config, then a system standard
    let thumbnail_dir = args
        .thumbnail_dir
        .or_else(|| config.thumbnail_dir.clone())
        .unwrap_or_else(|| {
            const THUMBNAIL_DIR_NAME: &str = "thumbs";
            let mut p = dirs.cache_dir().to_path_buf();
            p.push(THUMBNAIL_DIR_NAME);
            p
        });
    touch_dir(&thumbnail_dir)?;

    // Thumbnail size: Determined by the config, then a default
    const DEFAULT_THUMBNAIL_SIZE: u32 = 256;
    let thumbnail_size = config.thumbnail_size.unwrap_or(DEFAULT_THUMBNAIL_SIZE);

    // Font data: Determined by the config, then an embedded font
    let font_data = config
        .font
        .as_ref()
        .and_then(|f| f.path.as_ref())
        .and_then(|font_path| {
            fs::read(font_path)
                .map_err(|e| {
                    format!(
                        "Failed to read font file at `{}`: {}",
                        font_path.display(),
                        e
                    )
                })
                .print_err()
                .ok()
                .map(Cow::from)
        })
        .or_else(|| embedded_font().map(Cow::from))
        .ok_or_else(|| "Error: No font was provided\nEither provide a font in the config file or enable the embedded font".to_string())?;

    // Font size: Determined by the config, then a default
    const DEFAULT_FONT_SIZE: f32 = 25.0;
    let font_size = config
        .font
        .as_ref()
        .and_then(|f| f.size)
        .unwrap_or(DEFAULT_FONT_SIZE);

    let settings = Settings {
        rc_path,
        config_flags,
        thumbnail_dir,
        thumbnail_size,
        font_data,
        font_size,
    };

    // Run rlens
    rlens(paths, initial_index, settings)
}

/// Attempt to get the details of the config file
/// If the file does not exist, an empty config is returned
/// If reading or parsing the file fails, a descriptive error message is returned
fn get_config(path: &Path) -> Result<Config, String> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let config_data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file at `{}`: {}", path.display(), e))?;

    let config = toml::from_str(&config_data)
        .map_err(|e| format!("Error parsing config file at `{}`: {}", path.display(), e))?;

    Ok(config)
}

/// Get the data of the embedded font if enabled
/// While the font cannot meet every need, it is an acceptable default
fn embedded_font() -> Option<&'static [u8]> {
    #[cfg(feature = "embedded_font")]
    {
        let font_data = include_bytes!("../font/NotoSans-Regular.ttf");
        Some(font_data)
    }

    #[cfg(not(feature = "embedded_font"))]
    None
}

impl FromStr for ConfigFlag {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Infallible> {
        Ok(if let Some((name, val)) = s.split_once(':') {
            ConfigFlag {
                name: name.to_string(),
                val: Some(val.trim_start().to_string()),
            }
        } else {
            ConfigFlag {
                name: s.to_string(),
                val: None,
            }
        })
    }
}
