//! Module for the structure of the image viewer and high level drawing

use crate::gallery::Gallery;
use crate::geometry::*;
use crate::gfx::{CanvasExt, Font, Gfx};
use crate::image::{Image, LoadedImage, Metadata};
use crate::image_transform::{Align, ImageTransform, Scaling};
use crate::image_view::ImageView;
use crate::load_request::{FullRequest, ImageType, LoadRequest, ThumbnailRequest};
use crate::status_bar::{StatusBar, StatusBarPosition};
use crate::util::Offset;

use enum_map::Enum;
use femtovg::Color;
use std::path::PathBuf;

/// State of rlens
pub struct RLens {
    /// The current view
    mode: Mode,

    /// The image list
    /// Non-empty
    images: Box<[Image]>,

    /// The image view
    image_view: ImageView,
    /// Number of images to preload forwards
    preload_forward: usize,
    /// Number of images to preload backwards
    preload_backward: usize,
    /// Whether the status bar should be displayed in the image mode
    image_mode_status_bar: bool,

    /// The gallery
    gallery: Gallery,
    /// Whether to save generated thumbnails
    save_thumbnails: bool,

    /// The status bar
    status_bar: StatusBar,
    /// Position of the status bar
    status_bar_position: StatusBarPosition,

    /// The background color of rlens
    bg_color: Color,

    /// Whether draw requests should be ignored
    frozen: bool,
}

/// A mode in rlens
#[derive(Enum, Clone, Copy, PartialEq, Default, Debug)]
pub enum Mode {
    /// The image mode
    #[default]
    Image,
    /// The gallery mode
    Gallery,
}

/// rlens modes
pub const MODES: &[Mode] = &[Mode::Image, Mode::Gallery];

impl RLens {
    pub fn init(paths: Vec<PathBuf>, initial_index: usize) -> Self {
        assert!(!paths.is_empty());

        let images = paths.into_iter().map(Image::new_unloaded).collect();

        Self {
            mode: Mode::default(),

            images,

            image_view: ImageView::init(initial_index),
            preload_forward: 0,
            preload_backward: 0,
            image_mode_status_bar: false,

            gallery: Gallery::init(),
            save_thumbnails: false,

            status_bar: StatusBar::new(),
            status_bar_position: StatusBarPosition::default(),

            bg_color: Color::black(),

            frozen: false,
        }
    }
}

impl RLens {
    /// Get the current mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Set the mode
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Get the index of the current image for the mode
    pub fn current_image(&self) -> usize {
        match self.mode {
            Mode::Image => self.current_open_image(),
            Mode::Gallery => self.gallery_cursor(),
        }
    }

    /// Get the number of images in the image list
    pub fn total_images(&self) -> usize {
        self.images.len()
    }

    /// Get the details of the image at `index`
    /// Pre: `index` is valid
    pub fn get_image(&self, index: usize) -> &Image {
        &self.images[index]
    }

    /// Set whether rlens is frozen
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Unload the image at the index (both full and thumbnail)
    /// This will allow the load to be retried even if the image was marked unloadable
    /// Pre: `index` is valid
    pub fn unload_image(&mut self, index: usize, gfx: &mut Gfx) {
        let image = &mut self.images[index];

        image.full.unload_image(gfx);
        image.thumbnail.unload_image(gfx);

        image.forget_unloadable();
    }

    // === Image view ===

    /// Get the current image open in the image view
    pub fn current_open_image(&self) -> usize {
        self.image_view.current_image()
    }

    /// Set the current full image
    /// Pre: `index` is valid
    pub fn set_full_image(&mut self, index: usize, view: Size) {
        self.image_view.set_image(index, &self.images, view);
    }

    /// Reset the image view
    /// No effect if the current image is not loaded
    pub fn reset_image_view(&mut self, view: Size) {
        self.image_view.reset_if_loaded(&self.images, view);
    }

    /// The current transform
    pub fn transform(&mut self) -> Option<&mut ImageTransform> {
        self.image_view.transform()
    }

    pub fn scaling(&mut self) -> &mut Scaling {
        self.image_view.scaling()
    }

    pub fn align(&mut self) -> &mut Align {
        self.image_view.align()
    }

    pub fn image_mode_status_bar(&mut self) -> &mut bool {
        &mut self.image_mode_status_bar
    }

    pub fn set_preload_range(&mut self, forward: usize, backward: usize) {
        self.preload_forward = forward;
        self.preload_backward = backward;
    }

    // === Gallery ===

    /// Get the position of the gallery cursor
    pub fn gallery_cursor(&self) -> usize {
        self.gallery.cursor()
    }

    /// Set the cursor in the gallery
    /// Pre: `index` is valid
    pub fn set_gallery_cursor(&mut self, index: usize, view: Size, font: &Font) {
        self.gallery
            .set_cursor(index, self.gallery_size(view, font));
    }

    pub fn gallery_tiles_in_row(&self, view: Size, font: &Font) -> usize {
        self.gallery.tiles_in_row(self.gallery_size(view, font))
    }

    pub fn set_save_thumbnails(&mut self, save: bool) {
        self.save_thumbnails = save;
    }

    pub fn set_gallery_tile_width(&mut self, width: f32) {
        self.gallery.set_tile_width(width);
    }

    pub fn set_gallery_height_width_ratio(&mut self, ratio: f32) {
        self.gallery.set_height_width_ratio(ratio);
    }

    /// Calculate the size of the gallery
    fn gallery_size(&self, view: Size, font: &Font) -> Size {
        self.segment_bounds(view, font).negative_bar.size()
    }

    // === Status bar ===

    /// Set the text of the status bar
    pub fn set_status_bar(&mut self, text: (String, String)) {
        self.status_bar.set_text(text);
    }

    pub fn set_status_bar_position(&mut self, position: StatusBarPosition) {
        self.status_bar_position = position;
    }

    /// Whether the status bar is visible
    pub fn show_status_bar(&self) -> bool {
        match self.mode {
            Mode::Image => self.image_mode_status_bar,
            _ => true,
        }
    }

    // === Colors ===

    pub fn set_bg(&mut self, color: Color) {
        self.bg_color = color;
    }

    pub fn set_gallery_cursor_color(&mut self, color: Color) {
        self.gallery.set_cursor_color(color);
    }

    pub fn set_gallery_border_color(&mut self, color: Color) {
        self.gallery.set_border_color(color);
    }

    pub fn set_status_bar_color(&mut self, color: Color) {
        self.status_bar.set_bg(color);
    }
}

// === Image loading ===

impl RLens {
    /// Set a full image or thumbnail as loaded
    /// Returns whether a redraw is required and whether the current image was loaded
    pub fn set_loaded(
        &mut self,
        type_: ImageType,
        index: usize,
        loaded_image: LoadedImage,
        metadata: Metadata,
        view: Size,
    ) -> (bool, bool) {
        let image_size = loaded_image.size();

        // Update the image list
        {
            let image = &mut self.images[index];

            let load_state = match type_ {
                ImageType::Full => &mut image.full,
                ImageType::Thumbnail => &mut image.thumbnail,
            };
            load_state.load(loaded_image);

            image.metadata.set_loaded(metadata);
        }

        // Reset image view if we loaded the current open image
        if type_ == ImageType::Full && self.current_open_image() == index {
            self.image_view.reset_with_size(image_size, view);
        }

        // Determine whether to redraw and whether the current image was loaded
        match type_ {
            ImageType::Full if self.mode == Mode::Image => {
                let loaded_current = self.current_open_image() == index;
                let redraw = loaded_current;
                (redraw, loaded_current)
            }
            ImageType::Thumbnail if self.mode == Mode::Gallery => {
                let loaded_current = self.gallery_cursor() == index;
                let redraw = true;
                (redraw, loaded_current)
            }
            _ => (false, false),
        }
    }

    /// Mark an image as unloadable
    pub fn mark_unloadable(&mut self, index: usize) {
        self.images[index].mark_unloadable();
    }

    /// Poll for a load request
    /// Returns `None` if all images within the load range are already loaded
    pub fn poll_loads(&self, view: Size, font: &Font) -> Option<LoadRequest> {
        // Poll for the appropriate request type
        match self.mode {
            Mode::Image => self.poll_full_load().map(LoadRequest::Full),
            Mode::Gallery => self
                .poll_thumbnail_load(view, font)
                .map(LoadRequest::Thumbnail),
        }
    }

    /// Poll for a full load request
    fn poll_full_load(&self) -> Option<FullRequest> {
        self.image_offsets(self.current_open_image())
            // Filter to images within our load range
            .filter(|(_, offset, _)| offset.in_range(self.preload_forward, self.preload_backward))
            // Filter to images that are unloaded and not unloadable
            .filter(|&(_, _, image)| !image.full.is_loaded() && !image.is_unloadable())
            // Select the closest candidate
            .min_by_key(|(_, offset, _)| offset.key())
            // Make the request for this candidate
            .map(|(index, _, image)| FullRequest::for_image(index, image))
    }

    /// Poll for a thumbnail load request
    fn poll_thumbnail_load(&self, view: Size, font: &Font) -> Option<ThumbnailRequest> {
        // Load range
        let (first, tiles) = self
            .gallery
            .load_range(self.gallery_size(view, font))
            .unwrap_or((0, 0));

        self.image_offsets(first)
            // Filter to images within our load range
            .filter(|(_, offset, _)| offset.in_range(tiles, 0))
            // Filter to images that are unloaded and not unloadable
            .filter(|&(_, _, image)| !image.thumbnail.is_loaded() && !image.is_unloadable())
            // Select the closest candidate
            .min_by_key(|(_, offset, _)| offset.key())
            // Make the request for this candidate
            .map(|(index, _, image)| {
                ThumbnailRequest::for_image(index, image, self.save_thumbnails)
            })
    }

    /// Unload images that are out of the load range
    /// Acts on both full images and thumbnails
    pub fn unload_images(&mut self, gfx: &mut Gfx) {
        // Unload full images
        {
            let index = self.current_open_image();
            let load_forward = self.preload_forward;
            let load_backward = self.preload_backward;

            let unload = self
                .image_offsets_mut(index)
                // Filter to images outside the load range
                .filter(|(_, offset, _)| !offset.in_range(load_forward, load_backward))
                // Extract loaded images
                .filter_map(|(_, _, image)| image.full.unload());

            for loaded in unload {
                loaded.unload(gfx);
            }
        }

        // Unload thumbnails
        {
            // Load range
            let gallery_size = self.gallery_size(gfx.window.size(), &gfx.font);
            let (first, tiles) = self.gallery.load_range(gallery_size).unwrap_or((0, 0));

            let unload = self
                .image_offsets_mut(first)
                // Filter to images outside the load range
                .filter(|(_, offset, _)| !offset.in_range(tiles, 0))
                // Extract loaded thumbnails
                .filter_map(|(_, _, image)| image.thumbnail.unload());

            for loaded in unload {
                loaded.unload(gfx);
            }
        }
    }

    /// Iterator over the image list with offsets from a given index
    fn image_offsets(&self, index: usize) -> impl Iterator<Item = (usize, Offset, &Image)> {
        let length = self.total_images();
        self.images.iter().enumerate().map(move |(i, image)| {
            let offset = Offset::calculate(index, i, length);
            (i, offset, image)
        })
    }

    /// Iterator over the image list with offsets from a given index
    fn image_offsets_mut(
        &mut self,
        index: usize,
    ) -> impl Iterator<Item = (usize, Offset, &mut Image)> {
        let length = self.total_images();
        self.images.iter_mut().enumerate().map(move |(i, image)| {
            let offset = Offset::calculate(index, i, length);
            (index, offset, image)
        })
    }
}

// === Drawing ===

impl RLens {
    /// Draw the current state of rlens
    pub fn draw(&self, gfx: &mut Gfx, size: Size) {
        // Ignore the request if we are frozen
        if self.frozen {
            return;
        }

        let segment_bounds = self.segment_bounds(size, &gfx.font);

        // Clear the canvas
        gfx.canvas.clear();

        // Background
        gfx.canvas.draw_rect(segment_bounds.all, self.bg_color);

        // Main view
        match self.mode {
            Mode::Image => {
                self.image_view.draw(&self.images, gfx);
            }
            Mode::Gallery => {
                let bounds = segment_bounds.negative_bar;
                self.gallery.draw(&self.images, bounds, gfx);
            }
        }

        // Status bar
        if self.show_status_bar() {
            let bounds = segment_bounds.status_bar;
            self.status_bar.draw(bounds, gfx);
        }

        // Render to the window
        gfx.draw_frame();
    }
}

/// Bounds of segments within the view
struct SegmentBounds {
    /// The bounds of the entire view
    all: Rect,
    /// The bounds of the status bar
    status_bar: Rect,
    /// The bounds outside of the status bar
    negative_bar: Rect,
}

impl RLens {
    /// Calculate the segment bounds for the given window size
    fn segment_bounds(&self, size: Size, font: &Font) -> SegmentBounds {
        let all = Rect::from_size(size);

        let status_bar_height = StatusBar::height(font);

        let (bar_top, bar_bottom) = match self.status_bar_position {
            StatusBarPosition::Top => (0.0, status_bar_height),
            StatusBarPosition::Bottom => (size.height - status_bar_height, size.height),
        };
        let status_bar = Rect::new(Point::new(0.0, bar_top), Point::new(size.width, bar_bottom));

        let (negative_bar_top, negative_bar_bottom) = match self.status_bar_position {
            StatusBarPosition::Top => (bar_bottom, size.height),
            StatusBarPosition::Bottom => (0.0, bar_top),
        };
        let negative_bar = Rect::new(
            Point::new(0.0, negative_bar_top),
            Point::new(size.width, negative_bar_bottom),
        );

        SegmentBounds {
            all,
            status_bar,
            negative_bar,
        }
    }
}
