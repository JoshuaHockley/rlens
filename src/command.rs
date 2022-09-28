//! Module for rlens' internal command API.

use crate::command_types::{Color, ImageDetails, TransformDetails};
use crate::hooks::Hooks;
use crate::image_transform;
use crate::lua::{LuaContext, LuaResult};
use crate::program::{Program, Request, RequestSender};
use crate::rlens;
use crate::status_bar;
use crate::util::StrError;

use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::sync::mpsc::{sync_channel, SyncSender};

/// Run a command from the lua thread
pub fn run_command<C: Command>(
    cmd: C,
    request_tx: &RequestSender,
    lua_ctx: LuaContext,
) -> LuaResult<C::Output> {
    // Run the pre-lua
    let pre_lua_out = cmd.pre_lua(lua_ctx)?;

    // Build the command request
    let (output_tx, output_rx) = sync_channel(0);
    let cmd_req = CommandRequest {
        cmd,
        pre_lua_out,
        output_tx,
    };
    let req = Request::CommandRequest(Box::new(cmd_req));

    // Send to the main thread
    request_tx.send(req).ok();

    // Await the output response
    if let Ok(resp) = output_rx.recv() {
        let (out, hooks) = resp?;

        // Run the lua hooks
        hooks.run(lua_ctx);

        Ok(out)
    } else {
        // The channel was closed so the event loop has closed
        Err(StrError("Error: API use after exit".to_string()).into())
    }
}

/// A command that acts on `Program` and produces an output of type `Command::Output`
/// `Command::pre_lua` can be used to pass data obtained from lua into the main command
/// Commands can also set lua hooks to be run after the main command finishes
pub trait Command: Send + Debug + 'static {
    /// The type of the output of the command
    type Output: Send + Debug = ();

    /// Run the command on the program
    fn run(
        &self,
        program: &mut Program,
        hooks: &mut Hooks,
        pre_lua: Self::PreLuaOut,
    ) -> CommandResult<Self::Output>;

    /// The result of the pre-lua stage
    type PreLuaOut: Default + Send + Debug = ();

    /// The pre-lua stage of the command
    fn pre_lua(&self, _lua_ctx: LuaContext) -> LuaResult<Self::PreLuaOut> {
        // Default implementation for where pre-lua is not required (`PreLuaOut = ()`)
        Ok(Self::PreLuaOut::default())
    }
}

/// A request for a command's main body to be run
#[derive(Debug)]
struct CommandRequest<C: Command> {
    /// The command to be run
    cmd: C,
    /// The output of the command's pre-lua
    pre_lua_out: C::PreLuaOut,
    /// The sender to reply with the response to
    output_tx: SyncSender<CommandResult<(C::Output, Hooks)>>,
}

/// Trait to describe the capabilities of a command request
pub trait CommandRequestT: Send + Debug {
    fn handle(self: Box<Self>, program: &mut Program);
}

impl<C: Command> CommandRequestT for CommandRequest<C> {
    /// Handle the command request from the main thread
    fn handle(self: Box<Self>, program: &mut Program) {
        let mut hooks = Hooks::default();

        let out = self.cmd.run(program, &mut hooks, self.pre_lua_out);

        let output = out.map(|out| (out, hooks));

        self.output_tx.send(output).unwrap();
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug)]
pub enum CommandError {
    /// Image index out of scope (goto)
    ImageIndex(usize),
    /// Non-positive value where a positive value was expected
    NonPositive(f32),
    /// Zoom factor 0
    ZoomZero,
}

/// Command error display
impl Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CommandError::*;

        let error_msg = match self {
            ImageIndex(i) => format!("Image index `{}` was out of range", i),
            NonPositive(x) => format!("Expected a positive value, got `{}`", x),
            ZoomZero => "Cannot set zoom to 0".to_string(),
        };

        write!(f, "{}", error_msg)
    }
}

// `Error` implementation for packing into `rlua::Error`
impl Error for CommandError {}

// === Command utils ===

fn redraw(p: &mut Program) {
    p.draw();
}
fn redraw_image_view(p: &mut Program) {
    if p.rlens.mode() == rlens::Mode::Image {
        redraw(p);
    }
}
fn redraw_gallery(p: &mut Program) {
    if p.rlens.mode() == rlens::Mode::Gallery {
        redraw(p);
    }
}
fn redraw_status_bar(p: &mut Program) {
    if p.rlens.show_status_bar() {
        redraw(p);
    }
}

/// Set the mode
fn mode(target: rlens::Mode, p: &mut Program, hooks: &mut Hooks) {
    let current = p.rlens.mode();

    if current == target {
        return;
    }

    // Carry over current image from image mode to the gallery
    if current == rlens::Mode::Image && target == rlens::Mode::Gallery {
        let index = p.rlens.current_open_image();
        p.rlens
            .set_gallery_cursor(index, p.window_size(), &p.gfx.font);
    }

    p.rlens.set_mode(target);
    p.wake_image_loader();
    redraw(p);

    hooks.current_image_change();
}

/// Set the `current_image_change` hook if in the image mode
fn open_image_change(p: &mut Program, hooks: &mut Hooks) {
    if p.rlens.mode() == rlens::Mode::Image {
        hooks.current_image_change();
    }
}
/// Set the `current_image_change` hook if in the gallery mode
fn gallery_cursor_move(p: &mut Program, hooks: &mut Hooks) {
    if p.rlens.mode() == rlens::Mode::Gallery {
        hooks.current_image_change();
    }
}

/// Update the image transform
fn update_transform(
    update: impl FnOnce(&mut image_transform::ImageTransform),
    p: &mut Program,
    hooks: &mut Hooks,
) {
    if let Some(t) = p.rlens.transform() {
        update(t);

        redraw_image_view(p);

        hooks.transform_update();
    }
}

// === Command defs ===

#[derive(Debug)]
pub struct Exit;

impl Command for Exit {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.exit = true;
        Ok(())
    }
}

/// Change the current mode
#[derive(Debug)]
pub struct Mode(pub rlens::Mode);

impl Command for Mode {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        mode(self.0, p, hooks);
        Ok(())
    }
}

/// Get the current mode
#[derive(Debug)]
pub struct CurrentMode;

impl Command for CurrentMode {
    type Output = rlens::Mode;

    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<rlens::Mode> {
        Ok(p.rlens.mode())
    }
}

/// Select an image in the gallery to view in the image mode
#[derive(Debug)]
pub struct Select;

impl Command for Select {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        if p.rlens.mode() == rlens::Mode::Gallery {
            p.rlens
                .set_full_image(p.rlens.gallery_cursor(), p.window_size());
            mode(rlens::Mode::Image, p, hooks);
        }
        Ok(())
    }
}

/// Get the index of the current image
/// Index is always >= 1
#[derive(Debug)]
pub struct Index;

impl Command for Index {
    type Output = usize;

    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<usize> {
        Ok(p.rlens.current_image() + 1)
    }
}

/// Get the total number of images in the image list
#[derive(Debug)]
pub struct TotalImages;

impl Command for TotalImages {
    type Output = usize;

    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<usize> {
        Ok(p.rlens.total_images())
    }
}

/// Get the details of an image
/// Pre: `index` is valid
fn image_unchecked(index: usize, p: &mut Program) -> ImageDetails {
    let image = p.rlens.get_image(index);
    ImageDetails::collect(image)
}

/// Convert a position to a valid index
fn validate_position(pos: usize, p: &Program) -> CommandResult<usize> {
    if pos < 1 || pos > p.rlens.total_images() {
        return Err(CommandError::ImageIndex(pos));
    };

    Ok(pos - 1)
}

/// Get the details of the image at an index
#[derive(Debug)]
pub struct Image(pub usize);

impl Command for Image {
    type Output = ImageDetails;

    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<ImageDetails> {
        let index = validate_position(self.0, p)?;
        Ok(image_unchecked(index, p))
    }
}

/// Get the details of the current image
#[derive(Debug)]
pub struct CurrentImage;

impl Command for CurrentImage {
    type Output = ImageDetails;

    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<ImageDetails> {
        let index = p.rlens.current_image();
        Ok(image_unchecked(index, p))
    }
}

/// Trait for navigation of the image list common to the image view and gallery
trait ListNavigation {
    /// Set the current image by position (`1` for index `0`)
    fn goto(pos: usize, p: &mut Program, hooks: &mut Hooks) -> CommandResult<()>;

    /// Goto the next image in the image list
    /// No effect if there is no next image
    fn next(p: &mut Program, hooks: &mut Hooks);

    /// Goto the next image in the image list
    /// If at the last image, will wrap to the first
    fn next_wrapping(p: &mut Program, hooks: &mut Hooks);

    /// Goto the previous image in the image list
    /// No effect if there is no next image
    fn prev(p: &mut Program, hooks: &mut Hooks);

    /// Goto the previous image in the image list
    /// If at the first image, will wrap to the last
    fn prev_wrapping(p: &mut Program, hooks: &mut Hooks);

    /// Goto the first image
    fn first(p: &mut Program, hooks: &mut Hooks);

    /// Goto the last image
    fn last(p: &mut Program, hooks: &mut Hooks);
}

/// Trait for implementing `ListNavigation` generically
trait ListNavigationCore {
    /// Goto the index without validation
    fn goto_unchecked(index: usize, p: &mut Program, hooks: &mut Hooks);

    /// Get the current index in the image list
    fn current_index(p: &Program) -> usize;
}

/// Navigation in the image view
struct ImageViewNav;

impl ListNavigationCore for ImageViewNav {
    fn goto_unchecked(index: usize, p: &mut Program, hooks: &mut Hooks) {
        p.rlens.set_full_image(index, p.window_size());
        p.wake_image_loader();

        redraw_image_view(p);

        open_image_change(p, hooks);
    }

    fn current_index(p: &Program) -> usize {
        p.rlens.current_open_image()
    }
}

/// Navigation in the gallery
struct GalleryNav;

impl ListNavigationCore for GalleryNav {
    fn goto_unchecked(index: usize, p: &mut Program, hooks: &mut Hooks) {
        p.rlens
            .set_gallery_cursor(index, p.window_size(), &p.gfx.font);
        p.wake_image_loader();

        redraw_gallery(p);

        gallery_cursor_move(p, hooks);
    }

    fn current_index(p: &Program) -> usize {
        p.rlens.gallery_cursor()
    }
}

/// List navigation logic
impl<Nav: ListNavigationCore> ListNavigation for Nav {
    fn goto(pos: usize, p: &mut Program, hooks: &mut Hooks) -> CommandResult<()> {
        let index = validate_position(pos, p)?;

        if index != Nav::current_index(p) {
            Nav::goto_unchecked(index, p, hooks);
        }

        Ok(())
    }

    fn next(p: &mut Program, hooks: &mut Hooks) {
        let current_index = Nav::current_index(p);
        let last_index = p.rlens.total_images() - 1;

        if current_index < last_index {
            let new_index = current_index + 1;
            Nav::goto_unchecked(new_index, p, hooks);
        }
    }

    fn next_wrapping(p: &mut Program, hooks: &mut Hooks) {
        let current_index = Nav::current_index(p);
        let last_index = p.rlens.total_images() - 1;
        let new_index = if current_index < last_index {
            current_index + 1
        } else {
            0
        };

        if new_index != current_index {
            Nav::goto_unchecked(new_index, p, hooks);
        }
    }

    fn prev(p: &mut Program, hooks: &mut Hooks) {
        let current_index = Nav::current_index(p);

        if current_index > 0 {
            let new_index = current_index - 1;
            Nav::goto_unchecked(new_index, p, hooks);
        }
    }

    fn prev_wrapping(p: &mut Program, hooks: &mut Hooks) {
        let current_index = Nav::current_index(p);
        let new_index = if current_index > 0 {
            current_index - 1
        } else {
            let last_index = p.rlens.total_images() - 1;
            last_index
        };

        if new_index != current_index {
            Nav::goto_unchecked(new_index, p, hooks);
        }
    }

    fn first(p: &mut Program, hooks: &mut Hooks) {
        if Nav::current_index(p) != 0 {
            Nav::goto_unchecked(0, p, hooks);
        }
    }

    fn last(p: &mut Program, hooks: &mut Hooks) {
        let last_index = p.rlens.total_images() - 1;
        if Nav::current_index(p) != last_index {
            Nav::goto_unchecked(last_index, p, hooks);
        }
    }
}

#[derive(Debug)]
pub struct Goto(pub usize);

impl Command for Goto {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::goto(self.0, p, hooks)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Next;

impl Command for Next {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::next(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct NextWrapping;

impl Command for NextWrapping {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::next_wrapping(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Prev;

impl Command for Prev {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::prev(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct PrevWrapping;

impl Command for PrevWrapping {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::prev_wrapping(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct First;

impl Command for First {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::first(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Last;

impl Command for Last {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        ImageViewNav::last(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GalleryGoto(pub usize);

impl Command for GalleryGoto {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::goto(self.0, p, hooks)
    }
}

#[derive(Debug)]
pub struct GalleryNext;

impl Command for GalleryNext {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::next(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GalleryNextWrapping;

impl Command for GalleryNextWrapping {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::next_wrapping(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GalleryPrev;

impl Command for GalleryPrev {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::prev(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GalleryPrevWrapping;

impl Command for GalleryPrevWrapping {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::prev_wrapping(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GalleryFirst;

impl Command for GalleryFirst {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::first(p, hooks);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GalleryLast;

impl Command for GalleryLast {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        GalleryNav::last(p, hooks);
        Ok(())
    }
}

/// Move the gallery cursor vertically
/// `direction` is true for downwards movement
fn gallery_vertical_move(direction: bool, p: &mut Program, hooks: &mut Hooks) {
    let tiles_in_row = p.rlens.gallery_tiles_in_row(p.window_size(), &p.gfx.font);
    let current_index = GalleryNav::current_index(p);

    if let Some(new_index) = if direction {
        // Down
        let index = current_index + tiles_in_row;
        if index < p.rlens.total_images() {
            Some(index)
        } else {
            None
        }
    } else {
        // Up
        if current_index >= tiles_in_row {
            Some(current_index - tiles_in_row)
        } else {
            None
        }
    } {
        GalleryNav::goto_unchecked(new_index, p, hooks);

        redraw_gallery(p);
    }
}

/// Go up in the gallery
#[derive(Debug)]
pub struct GalleryUp;

impl Command for GalleryUp {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        gallery_vertical_move(false, p, hooks);
        Ok(())
    }
}

/// Go down in the gallery
#[derive(Debug)]
pub struct GalleryDown;

impl Command for GalleryDown {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        gallery_vertical_move(true, p, hooks);
        Ok(())
    }
}

/// Reset the image (as if it was just loaded)
#[derive(Debug)]
pub struct Reset;

impl Command for Reset {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.reset_image_view(p.window_size());
        redraw_image_view(p);

        hooks.transform_update();

        Ok(())
    }
}

/// Pan by (dx, dy)
#[derive(Debug)]
pub struct Pan(pub f32, pub f32);

impl Command for Pan {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        update_transform(|t| t.pan((self.0, self.1)), p, hooks);

        Ok(())
    }
}

/// Zoom by the given factor
/// `1` has no effect
/// `2` doubles the current zoom
/// `0.5` halves the current zoom
///
/// A negative factor has the inverse effect of its positive
/// `-2` halves the current zoom
///
/// Fails if factor is `0`
///
#[derive(Debug)]
pub struct Zoom(pub f32);

impl Command for Zoom {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        if self.0 == 0.0 {
            return Err(CommandError::ZoomZero);
        }

        let view = p.window_size();
        update_transform(|t| t.zoom(self.0, view), p, hooks);

        Ok(())
    }
}

/// Rotate clockwise by the given amount in degrees
#[derive(Debug)]
pub struct Rotate(pub f32);

impl Command for Rotate {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        let view = p.window_size();
        update_transform(|t| t.rotate(self.0, view), p, hooks);

        Ok(())
    }
}

/// Flip over the vertical axis
#[derive(Debug)]
pub struct HFlip;

impl Command for HFlip {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        let view = p.window_size();
        update_transform(|t| t.hflip(view), p, hooks);

        Ok(())
    }
}

/// Flip over the horizontal axis
#[derive(Debug)]
pub struct VFlip;

impl Command for VFlip {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        let view = p.window_size();
        update_transform(|t| t.vflip(view), p, hooks);

        Ok(())
    }
}

/// Set the pan from the top-left of the image
#[derive(Debug)]
pub struct SetPan(pub f32, pub f32);

impl Command for SetPan {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        update_transform(|t| t.set_pan((self.0, self.1)), p, hooks);

        Ok(())
    }
}

/// Set the zoom factor
/// Fails if factor is not positive
#[derive(Debug)]
pub struct SetZoom(pub f32);

impl Command for SetZoom {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        if self.0 <= 0.0 {
            return Err(CommandError::NonPositive(self.0));
        }

        update_transform(|t| t.set_zoom(self.0), p, hooks);

        Ok(())
    }
}

/// Set the clockwise rotation in degrees
#[derive(Debug)]
pub struct SetRotation(pub f32);

impl Command for SetRotation {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        update_transform(|t| t.set_rotation(self.0), p, hooks);

        Ok(())
    }
}

/// Set whether the image is flipped
#[derive(Debug)]
pub struct SetFlipped(pub bool);

impl Command for SetFlipped {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        update_transform(|t| t.set_flip(self.0), p, hooks);

        Ok(())
    }
}

/// Set the scaling mode
#[derive(Debug)]
pub struct Scaling(pub image_transform::Scaling);

impl Command for Scaling {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        *p.rlens.scaling() = self.0;
        Reset.run(p, hooks, ())
    }
}

/// Set the horizontal align
#[derive(Debug)]
pub struct AlignX(pub image_transform::AlignX);

impl Command for AlignX {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.align().x = self.0;
        Reset.run(p, hooks, ())
    }
}

/// Set the vertical align
#[derive(Debug)]
pub struct AlignY(pub image_transform::AlignY);

impl Command for AlignY {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.align().y = self.0;
        Reset.run(p, hooks, ())
    }
}

/// Get the current transform details
#[derive(Debug)]
pub struct Transform;

impl Command for Transform {
    type Output = Option<TransformDetails>;

    fn run(
        &self,
        p: &mut Program,
        _: &mut Hooks,
        _: (),
    ) -> CommandResult<Option<TransformDetails>> {
        Ok(p.rlens.transform().map(|t| TransformDetails::collect(t)))
    }
}

/// Reload the current image
#[derive(Debug)]
pub struct Reload;

impl Command for Reload {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        let index = p.rlens.current_image();
        p.rlens.unload_image(index, &mut p.gfx);
        p.wake_image_loader();
        redraw(p);
        Ok(())
    }
}

/// Set the preloading range for full images
#[derive(Debug)]
pub struct PreloadRange(pub usize, pub usize);

impl Command for PreloadRange {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_preload_range(self.0, self.1);
        Ok(())
    }
}

/// Set whether generated thumbnails should be saved
#[derive(Debug)]
pub struct SaveThumbnails(pub bool);

impl Command for SaveThumbnails {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_save_thumbnails(self.0);
        Ok(())
    }
}

/// Set the gallery tile width
/// `width` > 0
#[derive(Debug)]
pub struct GalleryTileWidth(pub f32);

impl Command for GalleryTileWidth {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        if self.0 <= 0.0 {
            return Err(CommandError::NonPositive(self.0));
        }

        p.rlens.set_gallery_tile_width(self.0);
        if p.rlens.mode() == rlens::Mode::Gallery {
            p.wake_image_loader();
        }
        redraw_gallery(p);

        Ok(())
    }
}

/// Set the gallery height-width ratio
/// `ratio` > 0
#[derive(Debug)]
pub struct GalleryHeightWidthRatio(pub f32);

impl Command for GalleryHeightWidthRatio {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        if self.0 <= 0.0 {
            return Err(CommandError::NonPositive(self.0));
        }

        p.rlens.set_gallery_height_width_ratio(self.0);
        if p.rlens.mode() == rlens::Mode::Gallery {
            p.wake_image_loader();
        }
        redraw_gallery(p);

        Ok(())
    }
}

/// Set whether the status bar is shown in image mode
#[derive(Debug)]
pub struct StatusBar(pub bool);

impl Command for StatusBar {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        *p.rlens.image_mode_status_bar() = self.0;
        redraw_image_view(p);
        Ok(())
    }
}

/// Toggle whether the status bar is shown in image mode
#[derive(Debug)]
pub struct ToggleStatusBar;

impl Command for ToggleStatusBar {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        let on = !*p.rlens.image_mode_status_bar();
        StatusBar(on).run(p, hooks, ())
    }
}

/// Refresh the status bar
#[derive(Debug)]
pub struct RefreshStatusBar;

impl Command for RefreshStatusBar {
    fn run(&self, p: &mut Program, _: &mut Hooks, text: (String, String)) -> CommandResult<()> {
        p.rlens.set_status_bar(text);

        redraw_status_bar(p);

        Ok(())
    }

    type PreLuaOut = (String, String);

    fn pre_lua(&self, lua_ctx: LuaContext) -> LuaResult<(String, String)> {
        // Query lua for the new status bar text
        lua_ctx
            .call_query::<(String, Option<String>)>("status_bar")
            .map(|text| {
                let (l, r) = text.unwrap_or_default();
                let r = r.unwrap_or_default();
                (l, r)
            })
    }
}

/// Set the position of the status bar
#[derive(Debug)]
pub struct StatusBarPosition(pub status_bar::StatusBarPosition);

impl Command for StatusBarPosition {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_status_bar_position(self.0);
        redraw_status_bar(p);
        Ok(())
    }
}

/// Set fullscreen either on or off
#[derive(Debug)]
pub struct FullScreen(pub bool);

impl Command for FullScreen {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.gfx.window.set_fullscreen(self.0);
        // No need to redraw as the resize will cause this
        Ok(())
    }
}

/// Toggle if the window is fullscreen
#[derive(Debug)]
pub struct ToggleFullScreen;

impl Command for ToggleFullScreen {
    fn run(&self, p: &mut Program, hooks: &mut Hooks, _: ()) -> CommandResult<()> {
        let on = !p.gfx.window.is_fullscreen();
        FullScreen(on).run(p, hooks, ())
    }
}

/// Freeze rlens
#[derive(Debug)]
pub struct Freeze;

impl Command for Freeze {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_frozen(true);
        Ok(())
    }
}

/// Unfreeze rlens and redraw
#[derive(Debug)]
pub struct Unfreeze;

impl Command for Unfreeze {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_frozen(false);
        redraw(p);
        Ok(())
    }
}

/// Set the background color of rlens
#[derive(Debug)]
pub struct BgColor(pub Color);

impl Command for BgColor {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_bg(self.0 .0);
        redraw(p);
        Ok(())
    }
}

/// Set the color of the image backdrop
#[derive(Debug)]
pub struct BackdropColor(pub Color);

impl Command for BackdropColor {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_backdrop_color(self.0 .0);
        redraw(p);
        Ok(())
    }
}

/// Set the highlight color of the cursor in the gallery
#[derive(Debug)]
pub struct GalleryCursorColor(pub Color);

impl Command for GalleryCursorColor {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_gallery_cursor_color(self.0 .0);
        redraw_gallery(p);
        Ok(())
    }
}

/// Set the color of the placeholder borders in the gallery
#[derive(Debug)]
pub struct GalleryBorderColor(pub Color);

impl Command for GalleryBorderColor {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_gallery_border_color(self.0 .0);
        redraw_gallery(p);
        Ok(())
    }
}

/// Set the background color of the status bar
#[derive(Debug)]
pub struct StatusBarColor(pub Color);

impl Command for StatusBarColor {
    fn run(&self, p: &mut Program, _: &mut Hooks, _: ()) -> CommandResult<()> {
        p.rlens.set_status_bar_color(self.0 .0);
        redraw_status_bar(p);
        Ok(())
    }
}
