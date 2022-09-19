//! Module for transforming a raw image by combining different components (such as 'pan' and 'rotation').
//! The transform can be updated with methods such as `pan` and `rotate`.
//! `transform` generates the final transform that can be applied to the raw image.
//!
//! When non-pan updates are made, the center of the view is fixed.
//! This takes the form of zooming in/out of the center, rotating about the center, and flipping
//! across the center.

use crate::geometry::*;

/// A transform on a raw image
#[derive(Default)]
pub struct ImageTransform {
    /// Offset of the top-left corner of the image from the origin
    pan: Vector,
    /// Zoom scale factor (> 0)
    zoom: f32,
    /// Angle of clockwise rotation (in [0, 360))
    rotation: f32,
    /// Whether the image is flipped (implemented as a horizontal flip)
    flip: bool,
}

/// A scaling mode based on the sizes of the image and view
#[derive(Default, Clone, Copy, Debug)]
pub enum Scaling {
    /// No scaling performed (zoom remains at 1)
    #[default]
    None,
    /// Fit the width of the image to the width of the window
    FitWidth,
    /// Fit the height of the image to the height of the window
    FitHeight,
    /// Fit the dimensions of the image within the dimensions of the window (leads to black bars)
    FitImage,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Align {
    pub x: AlignX,
    pub y: AlignY,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum AlignX {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum AlignY {
    #[default]
    Top,
    Center,
    Bottom,
}

impl ImageTransform {
    /// Generate an initial transform based on scaling and align options
    pub fn initial(scaling: Scaling, align: Align, image_size: Size, view: Size) -> Self {
        // Scaling
        let scale_factor = {
            let width_factor = view.width / image_size.width;
            let height_factor = view.height / image_size.height;

            use Scaling::*;
            match scaling {
                None => 1.0,
                FitWidth => width_factor,
                FitHeight => height_factor,
                FitImage => width_factor.min(height_factor), // Fit image by selecting the smaller factor
            }
        };

        // Align
        let align_pan = {
            // The size of the image post scaling
            // We need this to handle the align (we are aligning the scaled image)
            let scaled_image_size = image_size * scale_factor;

            let dx = view.width - scaled_image_size.width;
            let dy = view.height - scaled_image_size.height;

            let align_x = match align.x {
                AlignX::Left => 0.0,
                AlignX::Center => dx / 2.0,
                AlignX::Right => dx,
            };

            let align_y = match align.y {
                AlignY::Top => 0.0,
                AlignY::Center => dy / 2.0,
                AlignY::Bottom => dy,
            };

            Vector::new(align_x, align_y)
        };

        Self {
            pan: align_pan,
            zoom: scale_factor,
            rotation: 0.0,
            flip: false,
        }
    }

    /// Generate the transform to be applied to the image
    ///
    /// The input space should contain the raw image with its top-left corner at the origin
    /// The output space should contain the transformed image to be viewed with the origin at the top-left corner of the view
    ///
    pub fn transform(&self) -> Transform {
        // The components of the transform in the order they are performed
        let transforms = [
            self.zoom_t(),
            self.rotation_t(),
            self.flip_t(),
            self.pan_t(),
        ];

        transforms
            .iter()
            .fold(Transform::identity(), |acc, t| acc.then(t))
    }

    // === Transform components ===

    fn pan_t(&self) -> Transform {
        Translation::from(self.pan).to_transform()
    }

    fn zoom_t(&self) -> Transform {
        Transform::scale(self.zoom, self.zoom)
    }

    fn rotation_t(&self) -> Transform {
        Transform::rotation(Angle::degrees(self.rotation))
    }

    fn flip_t(&self) -> Transform {
        /// |-1  0  0 |
        /// | 0  1  0 |
        const HFLIP: Transform = Transform::new(-1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

        if self.flip {
            HFLIP
        } else {
            Transform::identity()
        }
    }

    // === Updates ===

    /// Update the transform and apply a correction so that the center of the
    /// view is fixed over the update
    fn with_fixed_center(&mut self, update: impl FnOnce(&mut Self), view: Size) {
        // The position of the center of the view
        let view_center = Rect::from_size(view).center();

        // The position in the untransformed image that is currently at the center of the view
        // We want to fix this point of the image in place
        let focus = self
            .transform()
            .inverse()
            .unwrap() // This is safe because we ensure each component is invertible
            .transform_point(view_center);

        // Perform the update (`self.transform()` will be affected)
        update(self);

        // The new position of the focus after the update
        let post_transform = self.transform().transform_point(focus);

        // Apply a correction to the pan so the focus is back at the center of the view
        let correction = view_center - post_transform;
        self.pan += correction;
    }

    pub fn pan(&mut self, pan: (f32, f32)) {
        self.pan += Vector::from(pan);
    }

    /// Pre: `factor` is non-zero
    pub fn zoom(&mut self, factor: f32, view: Size) {
        // Interpret a negative factor as the inverse of its positive
        let factor = if factor > 0.0 {
            factor
        } else {
            factor.abs().recip()
        };

        assert!(factor != 0.0);

        self.with_fixed_center(|t| t.zoom *= factor, view);
    }

    pub fn rotate(&mut self, degrees: f32, view: Size) {
        self.with_fixed_center(
            |t| {
                let dtheta = if t.flip { -degrees } else { degrees }; // when flipped invert our rotation
                t.rotation = (t.rotation + dtheta) % 360.0
            },
            view,
        );
    }

    pub fn hflip(&mut self, view: Size) {
        self.with_fixed_center(|t| t.flip = !t.flip, view);
    }

    pub fn vflip(&mut self, view: Size) {
        // Perform a vertical flip as a horizontal flip followed by an 180 degree rotation
        self.with_fixed_center(
            |t| {
                t.flip = !t.flip;
                t.rotation = (t.rotation + 180.0) % 360.0
            },
            view,
        );
    }

    // === Setters ===

    pub fn set_pan(&mut self, pan: (f32, f32)) {
        self.pan = Vector::from(pan);
    }

    /// Pre: `factor` is positive
    pub fn set_zoom(&mut self, factor: f32) {
        assert!(factor > 0.0);

        self.zoom = factor;
    }

    pub fn set_rotation(&mut self, degrees: f32) {
        self.rotation = degrees % 360.0
    }

    pub fn set_flip(&mut self, flip: bool) {
        self.flip = flip;
    }

    // === Getters ===

    pub fn get_pan(&self) -> (f32, f32) {
        self.pan.to_tuple()
    }

    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    pub fn get_flip(&self) -> bool {
        self.flip
    }
}
