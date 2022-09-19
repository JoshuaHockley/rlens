//! Module for the status bar

use crate::geometry::*;
use crate::gfx::{CanvasExt, Font, Gfx};
use crate::util::PrintErr;

use femtovg::{Align, Color};

pub struct StatusBar {
    /// The left text of the bar
    left_text: String,
    /// The right text of the bar
    right_text: String,
    /// The background color of the bar
    bg_color: Color,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum StatusBarPosition {
    Top,
    #[default]
    Bottom,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left_text: String::new(),
            right_text: String::new(),
            bg_color: Color::black(),
        }
    }

    /// Set the text of the status bar
    pub fn set_text(&mut self, text: (String, String)) {
        let (l, r) = text;
        self.left_text = l;
        self.right_text = r;
    }

    /// Draw the status bar within the bounds
    pub fn draw(&self, bounds: Rect, gfx: &mut Gfx) {
        let canvas = &mut gfx.canvas;
        let font = &gfx.font;

        // Draw the background
        canvas.draw_rect(bounds, self.bg_color);

        // Calculate total text bounds
        const SIDE_PADDING: f32 = 2.0;
        const SIDE_PADDING_VEC: Vector = Vector::new(SIDE_PADDING, 0.0);
        let text_bounds = Rect::new(bounds.min + SIDE_PADDING_VEC, bounds.max - SIDE_PADDING_VEC);

        // Draw the right text first with as much space as required
        let right_text_width = canvas
            .draw_text(&self.right_text, font, text_bounds, Align::Right)
            .print_err()
            .unwrap_or(0.0);

        // Draw the left text in the remaining space
        const TEXT_GAP: f32 = 10.0;
        let left_bounds = Rect::new(
            text_bounds.min,
            text_bounds.max - Vector::new(right_text_width + TEXT_GAP, 0.0),
        );
        canvas
            .draw_text(&self.left_text, font, left_bounds, Align::Left)
            .print_err()
            .ok();
    }

    /// Set the background color of the status bar
    pub fn set_bg(&mut self, color: Color) {
        self.bg_color = color;
    }

    pub fn height(font: &Font) -> f32 {
        font.height() + 3.0
    }
}
