//! Module for managing the image view of rlens

use crate::geometry::*;
use crate::gfx::{CanvasExt, Gfx};
use crate::image::{Image, LoadedImage};
use crate::image_transform::{Align, ImageTransform, Scaling};

use femtovg::Color;

pub struct ImageView {
    /// Index of the current image
    /// Valid index into `rlens.images`
    current_image: usize,

    /// The transform on the current image
    /// `None` only if the current image is not loaded
    transform: Option<ImageTransform>,
    /// Initial scaling
    scaling: Scaling,
    /// Initial align
    align: Align,
}

impl ImageView {
    pub fn init(index: usize) -> Self {
        Self {
            current_image: index,
            transform: None,
            scaling: Scaling::default(),
            align: Align::default(),
        }
    }
}

impl ImageView {
    /// Get the current image index
    pub fn current_image(&self) -> usize {
        self.current_image
    }

    /// Get the current loaded image
    /// `None` if the current image is not loaded
    fn current_loaded_image<'a>(&self, images: &'a [Image]) -> Option<&'a LoadedImage> {
        images[self.current_image].full.loaded()
    }

    /// Set the current image by index
    /// Pre: `index` is valid
    pub fn set_image(&mut self, index: usize, images: &[Image], view: Size) {
        // Update the index
        self.current_image = index;

        // Reset if the new image is loaded
        self.reset_if_loaded(images, view);
    }

    /// The current transform
    pub fn transform(&mut self) -> Option<&mut ImageTransform> {
        self.transform.as_mut()
    }

    /// The initial scaling
    pub fn scaling(&mut self) -> &mut Scaling {
        &mut self.scaling
    }

    /// The initial align
    pub fn align(&mut self) -> &mut Align {
        &mut self.align
    }

    /// Reset the image transform if the current image is loaded
    /// This should be called when the current image changes
    pub fn reset_if_loaded(&mut self, images: &[Image], view: Size) {
        if let Some(loaded_image) = self.current_loaded_image(images) {
            self.reset_with_size(loaded_image.size(), view)
        } else {
            // The current image is unloaded, so we have no transform
            self.transform = None;
        }
    }

    /// Reset the image transform for an image of the given size
    /// This should be called directly when the current image is loaded
    pub fn reset_with_size(&mut self, image_size: Size, view: Size) {
        self.transform = Some(ImageTransform::initial(
            self.scaling,
            self.align,
            image_size,
            view,
        ));
    }
}

// === Drawing ===

impl ImageView {
    /// Draw the image view if the current image is loaded
    pub fn draw(&self, images: &[Image], backdrop_color: Color, gfx: &mut Gfx) {
        if let Some(loaded_image) = self.current_loaded_image(images) {
            self.draw_image(loaded_image, backdrop_color, gfx);
        }
    }

    /// Draw the image view with the given image
    /// Pre: `image` is the current image
    fn draw_image(&self, image: &LoadedImage, backdrop_color: Color, gfx: &mut Gfx) {
        let canvas = &mut gfx.canvas;

        let id = image.id();
        let bounds = Rect::from_size(image.size());

        // Get the transform
        // The current image is loaded so the transform is present
        let transform = self
            .transform
            .as_ref()
            .expect("Transform was not present when drawing the loaded image")
            .transform();

        canvas.save_with(|canvas| {
            // Apply the current transform to the canvas
            canvas.set_transform_(transform);

            // Draw the backdrop
            canvas.draw_rect(bounds, backdrop_color);

            // Draw the image
            canvas.draw_image(id, bounds);
        });
    }
}
