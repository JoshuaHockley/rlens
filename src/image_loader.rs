//! Module for loading images and their metadata from the disk, and the image loader thread

use crate::gfx::Gfx;
use crate::image::{LoadedImage, Metadata};
use crate::load_request::{
    FullRequest, ImageType, LoadRequest, LoadRequestResponse, ThumbnailRequest,
};
use crate::program::{Request, RequestSender};
use crate::util::{hash_filepath, PrintErr};

use image::{io::Reader as ImageReader, DynamicImage, ImageFormat};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread::{spawn, JoinHandle};

/// Run the image loader thread
///
/// The thread will send the `ImageLoaderReady` request and then wait for a load request from the returned
/// sender.
/// The sender will block until this thread retrieves the request, so a load request should only be
/// made in response to the `ImageLoaderReady` request.
/// When a load request is received, the thread attempts to load the image, and then sends the
/// result to the main thread via the `ImageLoad` request.
///
/// When the sender is dropped, the thread will exit, and so can be safely joined
///
pub fn run_image_loader(
    request_tx: RequestSender,
    thumbnail_dir: PathBuf,
    thumbnail_size: u32,
) -> (SyncSender<LoadRequest>, JoinHandle<()>) {
    let (load_request_tx, load_request_rx) = sync_channel::<LoadRequest>(0);

    let thread = spawn(move || {
        loop {
            // Get a load request from the main thread
            request_tx.send(Request::ImageLoaderReady).ok();
            // Wait for the response, sleeping until loading is needed
            let req = if let Ok(r) = load_request_rx.recv() {
                // We have been sent a load request
                r
            } else {
                // The program is exiting, so return to be joined
                return;
            };

            // Handle the request
            if let Some(resp) = req.handle(&thumbnail_dir, thumbnail_size) {
                request_tx.send(Request::LoadImage(resp)).ok();
            } else {
                // The load failed so mark the source as unloadable
                let index = req.index();
                request_tx.send(Request::MarkUnloadable(index)).ok();
            }

            // Unload any out of range images
            request_tx.send(Request::UnloadImages).ok();
        }
    });

    (load_request_tx, thread)
}

impl LoadRequest {
    /// Handle a load request
    fn handle(&self, thumbnail_dir: &Path, thumbnail_size: u32) -> Option<LoadRequestResponse> {
        match self {
            LoadRequest::Full(details) => handle_full_request(details),
            LoadRequest::Thumbnail(details) => {
                handle_thumbnail_request(details, thumbnail_dir, thumbnail_size)
            }
        }
    }
}

fn handle_full_request(request: &FullRequest) -> Option<LoadRequestResponse> {
    // Load the full image
    let image = load_full(&request.details.path);

    image.map(|(image, metadata)| LoadRequestResponse {
        type_: ImageType::Full,
        index: request.details.index,
        image,
        metadata,
    })
}

/// Load a full image
fn load_full(path: &Path) -> Option<(Image, Metadata)> {
    Image::load(path).print_err().ok()
}

fn handle_thumbnail_request(
    request: &ThumbnailRequest,
    thumbnail_dir: &Path,
    thumbnail_size: u32,
) -> Option<LoadRequestResponse> {
    // Get the canonical path of the source image
    let src_path = request
        .details
        .path
        .canonicalize()
        .map_err(|e| {
            format!(
                "Error: Failed to obtain the canonical path of `{}`: {}",
                request.details.path.display(),
                e
            )
        })
        .print_err()
        .ok()?;

    // Get the path for the thumbnail
    let thumbnail_path = thumbnail_path(&src_path, thumbnail_dir);

    // Load / generate the thumbnail
    let thumbnail_result = {
        // Search for an existing thumbnail, and fallback to generating if not found
        let existing = || load_existing_thumbnail(&thumbnail_path, &src_path);
        let generated = || generate_thumbnail(&src_path, thumbnail_size);
        existing().or_else(generated)
    };

    thumbnail_result.map(
        |ThumbnailResult {
             thumbnail,
             metadata,
             generated,
         }| {
            // Loading was successful
            if generated && request.save {
                thumbnail.save(&thumbnail_path).print_err().ok();
            }

            LoadRequestResponse {
                type_: ImageType::Thumbnail,
                index: request.details.index,
                image: thumbnail,
                metadata,
            }
        },
    )
}

/// The result of loading a thumbnail
struct ThumbnailResult {
    thumbnail: Image,
    /// Metadata about the source image
    metadata: Metadata,
    /// Whether the thumbnail was generated
    generated: bool,
}

/// Try to load an existing thumbnail
/// Fails if the thumbnail cannot be loaded, or the source image has been modified since the
/// thumbnail's creation
fn load_existing_thumbnail(thumbnail_path: &Path, src_path: &Path) -> Option<ThumbnailResult> {
    if thumbnail_path.exists() {
        // Fail if the thumbnail is stale
        // (Assume not stale if we cannot determine this)
        let stale = check_stale_thumbnail(thumbnail_path, src_path).unwrap_or(false);
        if stale {
            return None;
        }

        // Try to load the thumbnail
        let (thumbnail, _) = Image::load(thumbnail_path).print_err().ok()?;

        // Extract the metadata for the source image
        let metadata = extract_metadata(src_path).print_err().ok()?;

        Some(ThumbnailResult {
            thumbnail,
            metadata,
            generated: false,
        })
    } else {
        None
    }
}

/// Generate a thumbnail for the image at `path`
fn generate_thumbnail(path: &Path, thumbnail_size: u32) -> Option<ThumbnailResult> {
    let (src, metadata) = Image::load(path).print_err().ok()?;
    let thumbnail = src.generate_thumbnail(thumbnail_size);
    Some(ThumbnailResult {
        thumbnail,
        metadata,
        generated: true,
    })
}

/// Get the thumbnail path for the image at `path`
/// Pre: `path` is absolute
fn thumbnail_path(path: &Path, thumbnail_dir: &Path) -> PathBuf {
    assert!(path.is_absolute());

    let hash_str = hash_filepath(path);

    let mut path = thumbnail_dir.to_path_buf();
    path.push(hash_str);
    path.set_extension("png");

    path
}

/// Check if a thumbnail is stale (i.e. The source has been modified since the thumbnail's creation)
/// Returns `None` if this could not be determined
fn check_stale_thumbnail(thumbnail: &Path, src: &Path) -> Option<bool> {
    let thumbnail_creation_time = fs::metadata(thumbnail).ok()?.created().ok()?;
    let src_mod_time = fs::metadata(src).ok()?.modified().ok()?;

    Some(src_mod_time.duration_since(thumbnail_creation_time).is_ok())
}

// === Image loading ===

/// A loaded image in memory
#[derive(Debug)]
pub struct Image(DynamicImage);

impl Image {
    /// Load an image and its metadata from a file
    fn load(path: &Path) -> Result<(Self, Metadata), String> {
        let reader = reader(path)?;

        let format = reader.format().and_then(format_str);

        let image = reader
            .decode()
            .map_err(|e| format!("Failed to decode image at `{}`: {}", path.display(), e))?;

        let dimensions = (image.width(), image.height());

        let metadata = Metadata { dimensions, format };

        Ok((Self(image), metadata))
    }

    /// Generate a thumbnail of the image
    /// The thumbnail fits within (`thumbnail_size` x `thumbnail_size`) and preserves the original aspect ratio
    fn generate_thumbnail(&self, thumbnail_size: u32) -> Self {
        Self(self.0.thumbnail(thumbnail_size, thumbnail_size))
    }

    /// Save the image to the given path
    fn save(&self, path: &Path) -> Result<(), String> {
        self.0
            .save_with_format(path, ImageFormat::Png)
            .map_err(|e| format!("Error: Failed to save image at `{}`: {}", path.display(), e))
    }

    /// Load the image into the canvas
    pub fn load_into_canvas(self, gfx: &mut Gfx) -> Result<LoadedImage, String> {
        // Convert to RGBA8
        let image = self.0.into_rgba8();

        let dimentions = image.dimensions();

        let image_data = image.into_vec();

        LoadedImage::register(&image_data, dimentions, gfx)
    }
}

/// Create an image reader for the file at `path`
fn reader(path: &Path) -> Result<ImageReader<BufReader<File>>, String> {
    let read_err = |e| format!("Failed to read image at `{}`: {}", path.display(), e);

    let mut reader = ImageReader::open(path)
        .map_err(read_err)?
        .with_guessed_format()
        .map_err(read_err)?;
    reader.no_limits();

    Ok(reader)
}

/// Extract the metadata about the image at the path
/// This should be used when the image itself will not be loaded
fn extract_metadata(path: &Path) -> Result<Metadata, String> {
    let reader = reader(path)?;

    let format = reader.format().and_then(format_str);

    let dimensions = reader.into_dimensions().map_err(|e| {
        format!(
            "Failed to extract the dimensions of `{}`: {}",
            path.display(),
            e
        )
    })?;

    Ok(Metadata {
        dimensions: dimensions,
        format,
    })
}

/// Get a string representation for an image format
/// e.g. "png"
fn format_str(format: ImageFormat) -> Option<&'static str> {
    format.extensions_str().first().cloned()
}
