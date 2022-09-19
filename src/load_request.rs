//! Module for the details of load requests and their responses

use crate::image::{Image, Metadata};
use crate::image_loader;

use std::path::PathBuf;

/// A request to load an image
#[derive(Debug)]
pub enum LoadRequest {
    Full(FullRequest),
    Thumbnail(ThumbnailRequest),
}

/// Common details of a load request
#[derive(Debug)]
pub struct LoadRequestCommon {
    /// Index of the image in the image list
    pub index: usize,
    /// The path of the source image
    //  Owning this greatly simplifies the image loader
    pub path: PathBuf,
}

/// A request to load a full image
#[derive(Debug)]
pub struct FullRequest {
    pub details: LoadRequestCommon,
}

/// A request to load a thumbnail
#[derive(Debug)]
pub struct ThumbnailRequest {
    pub details: LoadRequestCommon,
    /// Whether the thumbnail should saved if generated
    pub save: bool,
}

/// A successful response to a load request
#[derive(Debug)]
pub struct LoadRequestResponse {
    /// The type of image loaded
    pub type_: ImageType,
    /// Index of the image in the image list
    pub index: usize,
    /// The image data
    pub image: image_loader::Image,
    /// The metadata of the source image
    pub metadata: Metadata,
}

#[derive(PartialEq, Debug)]
pub enum ImageType {
    Full,
    Thumbnail,
}

impl LoadRequestCommon {
    pub fn for_image(index: usize, image: &Image) -> Self {
        Self {
            index,
            path: image.path().to_path_buf(),
        }
    }
}

impl FullRequest {
    pub fn for_image(index: usize, image: &Image) -> Self {
        Self {
            details: LoadRequestCommon::for_image(index, image),
        }
    }
}

impl ThumbnailRequest {
    pub fn for_image(index: usize, image: &Image, save: bool) -> Self {
        Self {
            details: LoadRequestCommon::for_image(index, image),
            save,
        }
    }
}

impl LoadRequest {
    /// Get the index associated with the request
    pub fn index(&self) -> usize {
        let details = match self {
            Self::Full(req) => &req.details,
            Self::Thumbnail(req) => &req.details,
        };
        details.index
    }
}
