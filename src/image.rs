//! Module for representing the images in the image list

use crate::geometry::*;
use crate::gfx::{CanvasExt, Gfx};

use femtovg::ImageId;
use std::mem;
use std::path::{Path, PathBuf};

/// An image in the image list
pub struct Image {
    /// The path of the source image
    path: PathBuf,
    /// Information about the full image
    pub full: LoadState<LoadedImage>,
    /// Information about the thumbnail
    pub thumbnail: LoadState<LoadedImage>,
    /// Metadata information
    pub metadata: LoadState<Metadata>,
    /// Whether the source is known to be unloadable
    unloadable: bool,
}

/// An item that may or may not be loaded
pub enum LoadState<T> {
    Unloaded,
    Loaded(T),
}

/// An image that has been loaded into the canvas
pub struct LoadedImage {
    /// The id under which the image is registered
    id: ImageId,
    /// The dimensions of the image
    size: Size,
}

/// Metadata for an image
#[derive(Clone, Debug)]
pub struct Metadata {
    /// The dimensions of the image: (width, height)
    pub dimensions: (u32, u32),
    /// A string representation of the format of the image
    /// e.g. "png"
    pub format: Option<&'static str>,
}

impl Image {
    pub fn new_unloaded(path: PathBuf) -> Self {
        Self {
            path,
            full: LoadState::Unloaded,
            thumbnail: LoadState::Unloaded,
            metadata: LoadState::Unloaded,
            unloadable: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the source image is known to be unloadable
    pub fn is_unloadable(&self) -> bool {
        self.unloadable
    }

    /// Mark the source image as unloadable
    pub fn mark_unloadable(&mut self) {
        self.unloadable = true;

        assert!(!self.full.is_loaded());
    }

    /// Forget if the image has been marked unloadable
    pub fn forget_unloadable(&mut self) {
        self.unloadable = false;
    }
}

impl<T> LoadState<T> {
    /// Get the loaded item
    pub fn loaded(&self) -> Option<&T> {
        match self {
            Self::Loaded(loaded) => Some(loaded),
            Self::Unloaded => None,
        }
    }

    fn take_loaded(self) -> Option<T> {
        match self {
            Self::Loaded(loaded) => Some(loaded),
            Self::Unloaded => None,
        }
    }

    /// Check if the item is loaded
    pub fn is_loaded(&self) -> bool {
        match self {
            Self::Loaded(_) => true,
            Self::Unloaded => false,
        }
    }

    /// Set the item as loaded and return the previously loaded item if possible
    pub fn set_loaded(&mut self, loaded: T) -> Option<T> {
        mem::replace(self, Self::Loaded(loaded)).take_loaded()
    }

    /// Set an unloaded item as loaded
    /// Panics if the item is already loaded
    pub fn load(&mut self, loaded: T) {
        assert!(
            self.set_loaded(loaded).is_none(),
            "Loaded over a loaded item"
        )
    }

    /// Unload the item and return the loaded item if possible
    pub fn unload(&mut self) -> Option<T> {
        mem::replace(self, Self::Unloaded).take_loaded()
    }
}

impl LoadState<LoadedImage> {
    /// Unload an image if loaded
    pub fn unload_image(&mut self, gfx: &mut Gfx) {
        if let Some(loaded) = self.unload() {
            loaded.unload(gfx);
        }
    }
}

impl LoadedImage {
    /// Register an image into the canvas
    /// * `image_data`: The image data in RGB8 pixels
    pub fn register(
        image_data: &[u8],
        dimentions: (u32, u32),
        gfx: &mut Gfx,
    ) -> Result<Self, String> {
        let id = gfx
            .canvas
            .register_image(image_data, dimentions)
            .map_err(|e| format!("Failed to create an image on the canvas: {}", e))?;

        let size = IntSize::from(dimentions).to_f32();

        Ok(Self { id, size })
    }

    pub fn id(&self) -> ImageId {
        self.id
    }

    pub fn size(&self) -> Size {
        self.size
    }

    /// Unload the image
    pub fn unload(self, gfx: &mut Gfx) {
        gfx.canvas.delete_image(self.id);
    }
}
