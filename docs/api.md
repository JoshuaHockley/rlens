# rlens lua API

### Main API
The following functions are found under the `rlens` table in lua. For example: `rlens.exit()`.

| Function | Description |
| :------- | :---------- |
| `exit()` | Exit rlens |
| `mode('image' \| 'gallery')` | Set the mode |
| `current_mode() -> 'image' \| 'gallery'` | Get the current mode |
| `select()` | Open the current gallery image in image mode |
| `index() -> int ` | Get the index of the current image |
| `total_images() -> int` | Get the number of images in the image list |
| `image(i: int) -> image_details` | Get the details of the image at index `i` |
| `current_image() -> image_details` | Get the details of the current image |
| `goto(i: int)` | Set the current open image in image mode to `i` |
| `next()` | Go to the next image in image mode |
| `next_wrapping()` | `next` but with wrapping to the start |
| `prev()` | Go to the previous image in image mode |
| `prev_wrapping()` | `prev` but with wrapping to the end |
| `first()` | Go to the first image in image mode |
| `last()` | Go to the last image in image mode |
| `gallery_goto(i: int)` | Set the index of the gallery cursor to `i` |
| `gallery_next()` | Go to the next image in the gallery  |
| `gallery_next_wrapping()` | `gallery_next` but with wrapping to the start |
| `gallery_prev()` | Go to the previous image in the gallery |
| `gallery_prev_wrapping()` | `gallery_prev` but with wrapping to the end |
| `gallery_first()` | Go to the first image in the gallery |
| `gallery_last()` | Go to the last image in the gallery |
| `gallery_up()` | Move the gallery cursor up |
| `gallery_down()` | Move the gallery cursor down |
| `reset()` | Reset the image transform (as if just loaded) |
| `pan(dx: num, dy: num)` | Adjust the pan by `(dx, dy)` |
| `zoom(f: num)` | Multiply the current zoom factor by `f` <br> Negative factors undo the effect of their positive |
| `rotate(d: num)` | Rotate the image clockwise `d` degrees |
| `hflip()` | Flip the image horizontally |
| `vflip()` | Flip the image vertically |
| `set_pan(x: num, y: num)` | Set the absolute pan from the top-left corner to `(x, y)` |
| `set_zoom(f: num)` | Set the zoom scale factor to `f` |
| `set_rotation(d: num)` | Set the absolute rotation to `d` degrees clockwise |
| `set_flipped(bool)` | Set whether the image is flipped |
| `scaling('none' \| 'fit_width' \| 'fit_height' \| 'fit')` | Set how images are initially scaled |
| `align_x('left' \| 'center' \| 'right')` | Set how images are initially aligned horizontally |
| `align_y('top' \| 'center' \| 'right')` | Set how images are initially aligned vertically |
| `transform() -> transform_details (nullable)` | Get details of the current transform |
| `reload()` | Reload the current image from file |
| `preload_range(forwards: int, backwards: int)` | Set the range at which images are preloaded |
| `save_thumbnails(bool)` | Set whether generated thumbnails are saved |
| `gallery_tile_width(num)` | Set the target width of tiles in the gallery |
| `gallery_height_width_ratio(num)` | Set the target ratio `height/width` of gallery tiles |
| `status_bar(bool)` | Set whether the status bar is shown in image mode |
| `toggle_status_bar()` | Toggle whether the status bar is shown in image mode |
| `refresh_status_bar()` | Refresh the text of the status bar (calls `query.status_bar`) |
| `status_bar_position('top' \| 'bottom')` | Set the position of the status bar |
| `fullscreen(bool)` | Set whether rlens is fullscreen |
| `toggle_fullscreen()` | Toggle whether rlens is fullscreen |
| `freeze()` | Freeze rlens - All drawing will be prevented until `unfreeze()` is called |
| `unfreeze()` | Unfreeze rlens and redraw |
| `bg_color(color)` | Set the background color of rlens |
| `backdrop_color(color)` | Set the color of the image backdrop |
| `gallery_cursor_color(color)` | Set the color of the gallery cursor |
| `gallery_border_color(color)` | Set the color of the borders of unloaded thumbnails in the gallery |
| `status_bar_color(color)` | Set the background color of the status bar |

#### Types
The above function signatures include some non-primitive types in lua. Here are their forms.
```
image_details {
    path: string,              The path of the image provided on startup
    absolute_path: string,     The absolute path of the image (nullable)
    filename: string,          The filename of the image (nullable)
    filestem: string,          The filestem of the image (nullable)
    metadata: {                The metadata of the image if loaded (nullable)
        dimentions: {
            width: int,        The width of the image in pixels
            height: int,       The height of the image in pixels
        },
        format: string,        The format of the image (e.g. 'png')
    },
}

transform_details {
    pan: {                     Pan from the top-left of the image
        x: num,
        y: num,
    },
    zoom: num,                 Zoom scale factor
    rotation: num,             Clockwise rotation in degrees
    flip: bool,                Whether the image is flipped
}

color {                        Components of a color (between 0 and 1 inclusive)
    r: num,
    g: num,
    b: num,
    a: num,                    Alpha component (0: transparent, 1: opaque)
}
```

### Keybinds
Keybinds are set with one of the 'bind' functions.

`bind(keycode, action)` sets a binding for all modes.\
`bind_image(keycode, action)` sets a binding for the image mode only.\
`bind_gallery(keycode, action)` sets a binding for the gallery mode only.

For example:\
`bind('q', rlens.exit)` binds the `q` key to exiting rlens.\
`bind_image('i', function() rlens.zoom(1.05) end)` binds the `i` key to zooming in while in the image mode.

See [keycodes](keycodes.md) for the form of `keycode` when modifiers or non-alphabetic keys are involved.

### Status bar
The text of the status bar is determined by the function `query.status_bar`, which should be defined in the lua rc like below:
```
function query.status_bar()
    ...
    return left[, right]
end
```
It should return one or two strings. The first to be aligned on the left of the status bar, and the second on the right.

The rc is also responsible for keeping the status bar up to date - rlens won't refresh it by itself.
This should be done by attaching a call to `rlens.refresh_status_bar()` to relevant [hooks](#hooks), which will itself call `query.status_bar()`.

### Hooks
Hooks are lua functions defined in the rc that are called by rlens on certain events.
Their primary purpose is to refresh the status bar, but they can also be used to support other behaviour.

Hooks are searched for under the `hooks` table and should be defined like below:
```
function hook.resize()
    ...
end
```

| Hook                   | Trigger |
| :--------------------- | :------ |
| `current_image_change` | The current image is changed (e.g. `next`, `goto`) |
| `transform_update`     | The transform is updated (e.g. `pan`, `reset`) |
| `current_image_load`   | The current image (which was unloaded) is loaded <br> This applies to thumbnails when in gallery mode |
| `resize`               | The window is resized |

### Config flags

Config flags provide a way for passing custom command line options into the lua rc.

If rlens is launched as `rlens --flag foo:bar ...` then when the rc is run, `flag.foo` will have the value `'bar'`.
If no value is provided (`rlens --flag foo`), then the value in lua will be `true`.
To check if a flag has been set (with or without a value), an if statement suffices: `if (flag.foo) ...`.

As an example, here is how config flags can be used to opt-out of thumbnail saving at the command line:
```
save = true
if (flag.private) then
   save = false
end

rlens.save_thumbnails(save)
```
`rlens --flag private ...` will now disable saving new thumbnails
