//! Module for the gallery mode

use crate::geometry::*;
use crate::gfx::{CanvasExt, Gfx};
use crate::image::{Image, LoadedImage};

use femtovg::Color;

pub struct Gallery {
    /// The current position in the gallery
    /// Valid index into `images`
    cursor: usize,
    /// Index of an image in the top row of the gallery grid
    /// Used to track the current 'scroll' of the gallery
    anchor: usize,

    /// The max width of the thumbnail tiles in pixels
    /// The width will be reduced to fit the size of the window
    /// `> 0`
    tile_width: f32,
    /// The target ratio of tiles height to their width
    /// The ratio will be adjusted to fit the size of the window
    /// `> 0`
    /// `1` will aim to produce square tiles
    height_width_ratio: f32,

    /// The color to highlight the current tile with
    cursor_color: Color,
    /// The color of the placeholder borders
    placeholder_border_color: Color,
}

impl Gallery {
    pub fn init() -> Self {
        Self {
            cursor: 0,
            anchor: 0,
            tile_width: 200.0,
            height_width_ratio: 1.0,
            cursor_color: Color::white(),
            placeholder_border_color: Color::white(),
        }
    }
}
impl Gallery {
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set the cursor in the gallery
    /// Pre: `index` is valid
    pub fn set_cursor(&mut self, index: usize, view: Size) {
        self.update_anchor(index, view);

        self.cursor = index;
    }

    /// Update the anchor to satisfy a new cursor index
    /// Pre: `index` is valid
    fn update_anchor(&mut self, index: usize, view: Size) {
        let tiling = match self.tiling(view) {
            Some(t) => t,
            _ => {
                // No tiling so fix the anchor on the new index
                self.anchor = index;
                return;
            }
        };

        let (first, tiles) = self.visible_range(&tiling);
        let last = first + tiles - 1;

        // The row containing `index`
        let index_row = index / tiling.tiles_in_row;

        if index < first {
            // The current image is above the visible range
            // Decrease the anchor to place the image in the top row
            self.anchor = index_row * tiling.tiles_in_row;
        } else if index > last {
            // The current image is below the visible range
            // Increase the anchor to place the image in the bottom row
            self.anchor = (index_row - tiling.tiles_in_col + 1) * tiling.tiles_in_row;
        } else {
            // The image is within the visible range
            // Retain the current anchor
        }
    }

    /// Set the tile width
    /// Pre: `width` > 0
    pub fn set_tile_width(&mut self, width: f32) {
        assert!(width > 0.0);
        self.tile_width = width;
    }

    /// Set the height width ratio of the tiles
    /// Pre: `ratio` > 0
    pub fn set_height_width_ratio(&mut self, ratio: f32) {
        assert!(ratio > 0.0);
        self.height_width_ratio = ratio;
    }

    pub fn set_cursor_color(&mut self, color: Color) {
        self.cursor_color = color;
    }

    pub fn set_border_color(&mut self, color: Color) {
        self.placeholder_border_color = color;
    }

    /// Calculate the number of tiles in a row of the gallery
    pub fn tiles_in_row(&self, view: Size) -> usize {
        self.tiling(view).map(|t| t.tiles_in_row).unwrap_or(0)
    }

    /// Get the range of images that should be loaded
    /// Returns the index of the first image, and the total number of images to load
    pub fn load_range(&self, view: Size) -> Option<(usize, usize)> {
        let tiling = self.tiling(view)?;
        Some(self.visible_range(&tiling))
    }
}

// === Tiling ===

/// Gallery tiling details
struct Tiling {
    /// The number of tiles in a row of the gallery
    /// > 0
    tiles_in_row: usize,
    /// The number of tiles in a column of the gallery (i.e. number of rows)
    /// > 0
    tiles_in_col: usize,

    /// The width of a tile
    tile_width: f32,
    /// The height of a tile
    tile_height: f32,
}

impl Gallery {
    /// Calculate the tiling of the gallery within the given view size
    fn tiling(&self, view: Size) -> Option<Tiling> {
        let tiles_in_row = (view.width / self.tile_width).round() as usize;
        if tiles_in_row == 0 {
            return None;
        }

        let tile_width = view.width / tiles_in_row as f32;

        let tiles_in_col = (view.height / (tile_width * self.height_width_ratio)).round() as usize;
        if tiles_in_col == 0 {
            return None;
        }

        let tile_height = view.height / tiles_in_col as f32;

        Some(Tiling {
            tiles_in_row,
            tiles_in_col,
            tile_width,
            tile_height,
        })
    }
}

impl Gallery {
    /// Get the range of images that are visible in the gallery
    /// Returns the index of the first visible tile, and the number of visible tiles
    fn visible_range(&self, tiling: &Tiling) -> (usize, usize) {
        let first = self.anchor - self.anchor % tiling.tiles_in_row;
        let tiles = tiling.tiles_in_row * tiling.tiles_in_col;
        (first, tiles)
    }
}

// === Drawing ===

impl Gallery {
    /// Draw the gallery
    pub fn draw(&self, images: &[Image], bounds: Rect, gfx: &mut Gfx) {
        let tiling = match self.tiling(bounds.size()) {
            Some(t) => t,
            _ => {
                return;
            }
        };

        // The offset of the top-left tile within the view
        let grid_offset = bounds.min.to_vector();

        let tile_bounds = Rect::from_size(Size::new(tiling.tile_width, tiling.tile_height));

        // Calculate the offset of a tile from the view
        let tile_offset =
            |row, col| tile_offset(row, col, tiling.tile_width, tiling.tile_height) + grid_offset;

        /// Width of the gap around the thumbnail areas (inner tiles)
        const INNER_TILE_GAP: f32 = 5.0;
        let inner_tile_bounds = tile_bounds.inner_box(SideOffsets::new_all_same(INNER_TILE_GAP));
        if inner_tile_bounds.is_empty() {
            return;
        }

        let (first, tiles) = self.visible_range(&tiling);

        // Iterator over the tile coordinates (row, col) in row-major order
        let tile_coords = (0..)
            .map(|row| (0..tiling.tiles_in_row).map(move |col| (row, col)))
            .flatten()
            // Cut off 'offscreen' tiles
            .take(tiles)
            .collect::<Vec<_>>();

        // Iterator over the bounds of the inner tiles in row-wise order
        let inner_tiles = tile_coords
            .iter()
            // Calculate bounds of inner tiles
            .map(|&(row, col)| {
                let offset = tile_offset(row, col);
                inner_tile_bounds.translate(offset)
            });

        // Highlight the current tile
        {
            if let Some(&(row, col)) = tile_coords.get(self.cursor - first) {
                let offset = tile_offset(row, col);
                let current_tile = tile_bounds.translate(offset);

                highlight_tile(current_tile, self.cursor_color, gfx);
            }
        }

        // Iterator over the thumbnails for each visible image
        let thumbnails = images
            .iter()
            .skip(first)
            .take(tiles)
            .map(|image| &image.thumbnail);

        // Draw the tiles
        for (thumbnail, inner_tile) in thumbnails.zip(inner_tiles) {
            if let Some(thumbnail) = thumbnail.loaded() {
                // Draw the thumbnail in the inner tile
                draw_thumbnail(thumbnail, inner_tile, gfx);
            } else {
                // The thumbnail is not loaded so draw a placeholder instead
                draw_placeholder(inner_tile, self.placeholder_border_color, gfx);
            }
        }
    }
}

/// Draw the thumbnail within the given bounds, centered and not stretched
fn draw_thumbnail(thumbnail: &LoadedImage, bounds: Rect, gfx: &mut Gfx) {
    let thumbnail_size = thumbnail.size();

    if thumbnail_size.is_empty() {
        return;
    }

    // The scale factor required to fit the thumbnail within the bounds
    let scale_factor =
        (bounds.width() / thumbnail_size.width).min(bounds.height() / thumbnail_size.height);

    // The size of the thumbnail after scaling
    let scaled_thumbnail_size = thumbnail_size * scale_factor;

    // The offset of the thumbnail to be centrally aligned in the bounds
    let offset = Vector::new(
        (bounds.width() - scaled_thumbnail_size.width) / 2.0,
        (bounds.height() - scaled_thumbnail_size.height) / 2.0,
    );

    // The true bounds of the thumbnail within the tile
    let image_bounds = Rect::from_origin_and_size(bounds.min + offset, scaled_thumbnail_size);

    // Draw the thumbnail
    gfx.canvas.draw_image(thumbnail.id(), image_bounds);
}

/// Highlight the tile given its bounds
fn highlight_tile(bounds: Rect, highlight: Color, gfx: &mut Gfx) {
    gfx.canvas.draw_rect(bounds, highlight);
}

/// Draw a placeholder icon in the inner bounds of a tile
fn draw_placeholder(bounds: Rect, border_color: Color, gfx: &mut Gfx) {
    /// Width of the placeholder border
    const BORDER_WIDTH: f32 = 2.0;

    if bounds.width() < BORDER_WIDTH * 2.0 || bounds.height() < BORDER_WIDTH * 2.0 {
        return;
    }

    gfx.canvas
        .draw_rect_outline(bounds, BORDER_WIDTH, border_color);
}

/// Calculate the offset of a tile from the top-left of the grid
/// The top-left tile has an offset of 0
fn tile_offset(row: usize, col: usize, tile_width: f32, tile_height: f32) -> Vector {
    let x = col as f32 * tile_width;
    let y = row as f32 * tile_height;

    Vector::new(x, y)
}
