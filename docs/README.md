# rlens
A lightweight, scriptable image viewer written in Rust.

### Usage
Run rlens with a list of image paths.\
`rlens a.png b.jpg c.webp`

Alternatively, paths can be provided by stdin.\
`find -type f | rlens` will open all the files in the current directory.

### Features
* Wide support of image formats (see [image-rs](https://github.com/image-rs/image#supported-image-formats))
* Basic image manipulation (pan, zoom, rotate, flip)
* A gallery of thumbnails for browsing
* Preloading of surrounding images
* A scriptable status bar
* [lua](https://www.lua.org/) based configuration
* Cross-platform (including Wayland)
* Pure rust - No external dependencies required

#### Future
* Simple command line for interacting with rlens without keybinds (e.g. `:goto 4`)
* Slideshow

#### Not provided
* Animation support - When viewing animated image files rlens shows only the first frame

### Configuration
rlens is configured by two files:
* `config.toml`: A toml file specifying settings required on startup
* `rc.lua`: A lua file for runtime configuration, including keybinds

These files are searched for in the config directory, which can be set by the command-line option `--config-dir`, or the environment variable `RLENS_CONFIG_DIR`.\
If neither are set a system default is used (`~/.config/rlens/`, `$HOME/Library/Application Support/rlens`, or `...\AppData\Roaming\rlens\config`).

To get started you'll need the default configuration.
Copy the contents of [this directory](../config) to your chosen config directory and rlens will be ready to use. See the [README](../config/README.md) for a list of the set keybinds.

If you want to edit the rc, or script your own extensions to rlens, you'll want to read the [lua API reference](api.md).

### rlens-folder
A small python utility is provided for opening all the images in a file's directory, starting at that file.\
`rlens-folder images/b.jpg` will run `rlens --start-at 2 a.png b.jpg c.webp`.

This is intended to be used as the default image opener for file explorers, for more smoothly browsing a folder of images.

