//! Module for custom types that commands accept and output
//!
//! The key role of this module is to implement `rlua::{FromLua, ToLua}` on the
//! input / output types respectively

use crate::image::{Image, Metadata};
use crate::image_transform::{AlignX, AlignY, ImageTransform, Scaling};
use crate::rlens::Mode;
use crate::status_bar::StatusBarPosition;
use crate::util::StrError;

use rlua::prelude::{LuaError, LuaResult};
use rlua::{Context, FromLua, ToLua, Value};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::str::FromStr;

impl FromLua<'_> for Mode {
    fn from_lua(v: Value, _: Context) -> LuaResult<Self> {
        parse_lua_str(v)
    }
}

impl FromStr for Mode {
    type Err = StrError;

    fn from_str(s: &str) -> Result<Self, StrError> {
        match s {
            "image" => Ok(Self::Image),
            "gallery" => Ok(Self::Gallery),

            _ => Err(StrError("Invalid mode".to_string())),
        }
    }
}

impl ToLua<'_> for Mode {
    fn to_lua(self, ctx: Context) -> LuaResult<Value> {
        match self {
            Self::Image => "image",
            Self::Gallery => "gallery",
        }
        .to_lua(ctx)
    }
}

/// Details of an image
#[derive(Debug)]
pub struct ImageDetails {
    /// The path of the image as provided to rlens
    path: PathBuf,
    /// The absolute path of the image
    absolute_path: Option<PathBuf>,
    /// The filename of the image
    filename: Option<OsString>,
    /// The stem of the filename
    filestem: Option<OsString>,
    /// Metadata of the image
    metadata: Option<Metadata>,
}

impl ToLua<'_> for ImageDetails {
    fn to_lua(self, ctx: Context) -> LuaResult<Value> {
        let t = ctx.create_table()?;

        t.set("path", pathbuf_to_string(self.path))?;
        t.set(
            "absolute_path",
            self.absolute_path.and_then(pathbuf_to_string),
        )?;
        t.set("filename", self.filename.and_then(os_string_to_string))?;
        t.set("filestem", self.filestem.and_then(os_string_to_string))?;
        t.set("metadata", self.metadata)?;

        Ok(Value::Table(t))
    }
}

impl ImageDetails {
    /// Collect relevant details from an `Image`
    pub fn collect(image: &Image) -> Self {
        Self {
            path: image.path().to_path_buf(),
            absolute_path: image.path().canonicalize().ok(),
            filename: image.path().file_name().map(OsStr::to_os_string),
            filestem: image.path().file_stem().map(OsStr::to_os_string),
            metadata: image.metadata.loaded().cloned(),
        }
    }
}

impl ToLua<'_> for Metadata {
    fn to_lua(self, ctx: Context) -> LuaResult<Value> {
        let t = ctx.create_table()?;

        t.set("dimensions", Dimensions(self.dimensions))?;
        t.set("format", self.format)?;

        Ok(Value::Table(t))
    }
}

struct Dimensions((u32, u32));
impl ToLua<'_> for Dimensions {
    fn to_lua(self, ctx: Context) -> LuaResult<Value> {
        let (width, height) = self.0;

        let t = ctx.create_table()?;

        t.set("width", width)?;
        t.set("height", height)?;

        Ok(Value::Table(t))
    }
}

/// Details of the image transform
#[derive(Debug)]
pub struct TransformDetails {
    /// Pan from the top-left
    pan: Pan,
    /// Zoom factor
    zoom: f32,
    /// Clockwise rotation in degrees
    rotation: f32,
    /// Whether the image is flipped
    flip: bool,
}

impl ToLua<'_> for TransformDetails {
    fn to_lua(self, ctx: Context) -> LuaResult<Value> {
        let t = ctx.create_table()?;

        t.set("pan", self.pan)?;
        t.set("zoom", self.zoom)?;
        t.set("rotation", self.rotation)?;
        t.set("flip", self.flip)?;

        Ok(Value::Table(t))
    }
}

impl TransformDetails {
    pub fn collect(t: &ImageTransform) -> Self {
        let pan = {
            let (x, y) = t.get_pan();
            Pan { x, y }
        };
        let zoom = t.get_zoom();
        let rotation = t.get_rotation();
        let flip = t.get_flip();

        Self {
            pan,
            zoom,
            rotation,
            flip,
        }
    }
}

#[derive(Debug)]
pub struct Pan {
    x: f32,
    y: f32,
}

impl ToLua<'_> for Pan {
    fn to_lua(self, ctx: Context) -> LuaResult<Value> {
        let t = ctx.create_table()?;

        t.set("x", self.x)?;
        t.set("y", self.y)?;

        Ok(Value::Table(t))
    }
}

impl FromLua<'_> for Scaling {
    fn from_lua(v: Value, _: Context) -> LuaResult<Self> {
        parse_lua_str(v)
    }
}

impl FromStr for Scaling {
    type Err = StrError;

    fn from_str(s: &str) -> Result<Self, StrError> {
        match s {
            "none" => Ok(Self::None),
            "fit_width" => Ok(Self::FitWidth),
            "fit_height" => Ok(Self::FitHeight),
            "fit" => Ok(Self::FitImage),

            _ => Err(StrError(format!("Invalid scaling `{}`", s))),
        }
    }
}

impl FromLua<'_> for AlignX {
    fn from_lua(v: Value, _: Context) -> LuaResult<Self> {
        parse_lua_str(v)
    }
}

impl FromStr for AlignX {
    type Err = StrError;

    fn from_str(s: &str) -> Result<Self, StrError> {
        match s {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),

            _ => Err(StrError(format!("Invalid X align `{}`", s))),
        }
    }
}

impl FromLua<'_> for AlignY {
    fn from_lua(v: Value, _: Context) -> LuaResult<Self> {
        parse_lua_str(v)
    }
}

impl FromStr for AlignY {
    type Err = StrError;

    fn from_str(s: &str) -> Result<Self, StrError> {
        match s {
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),

            _ => Err(StrError(format!("Invalid Y align `{}`", s))),
        }
    }
}

impl FromLua<'_> for StatusBarPosition {
    fn from_lua(v: Value, _: Context) -> LuaResult<Self> {
        parse_lua_str(v)
    }
}

impl FromStr for StatusBarPosition {
    type Err = StrError;

    fn from_str(s: &str) -> Result<Self, StrError> {
        match s {
            "top" => Ok(Self::Top),
            "bottom" => Ok(Self::Bottom),

            _ => Err(StrError(format!("Invalid position `{}`", s))),
        }
    }
}

/// Wrapper around `femtovg::Color` for `FromLua` implementation
#[derive(Debug)]
pub struct Color(pub femtovg::Color);

impl FromLua<'_> for Color {
    /// Convert from a table representation of a color to the internal rust type
    /// Table representation:
    ///     `{ r = _, g = _, b = _, a = _ }`
    ///   where `_` are `number`s between `0` and `1` inclusive
    fn from_lua(v: Value, _: Context) -> LuaResult<Self> {
        let t = match v {
            Value::Table(t) => Ok(t),
            _ => Err(StrError(format!(
                "Expected a table, found {}",
                v.type_name()
            ))),
        }?;

        let r = t.get("r")?;
        let g = t.get("g")?;
        let b = t.get("b")?;
        let a = t.get("a")?;

        // Validate components
        for comp in [r, g, b, a] {
            if comp < 0.0 || comp > 1.0 {
                // Invalid value
                return Err(StrError(format!(
                    "`{}` is not a valid color component (must be between 0 and 1 inclusive)",
                    comp
                ))
                .into());
            }
        }

        Ok(Self(femtovg::Color::rgbaf(r, g, b, a)))
    }
}

/// Parse a lua value expected to be a string
fn parse_lua_str<T, E>(v: Value) -> LuaResult<T>
where
    T: FromStr<Err = E>,
    LuaError: From<E>,
{
    // Check the value is a string
    let s = match v {
        Value::String(s) => Ok(s),
        _ => Err(StrError(format!(
            "Expected a string, found {}",
            v.type_name()
        ))),
    }?;

    // Convert to `&str`
    let s = s.to_str()?;

    // Parse
    let res = s.parse()?;

    Ok(res)
}

fn pathbuf_to_string(p: PathBuf) -> Option<String> {
    p.into_os_string().into_string().ok()
}

fn os_string_to_string(s: OsString) -> Option<String> {
    s.into_string().ok()
}
