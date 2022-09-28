//! Module for graphics infrastructure with `femtovg`

use crate::geometry::*;
use crate::window::Window;

use femtovg::imgref::Img;
use femtovg::rgb::AsPixels;
use femtovg::{
    renderer, Align, Baseline, Color, FontId, ImageFlags, ImageId, ImageSource, Paint, Path,
};

/// Graphics Api
pub struct Gfx {
    /// femtovg canvas
    pub canvas: Canvas,
    /// Font handle
    pub font: Font,

    /// The window and the GL context
    pub window: Window,
}

type Canvas = femtovg::Canvas<renderer::OpenGl>;

/// Loaded font details
pub struct Font {
    id: FontId,
    size: f32,
    height: f32,
}

pub type FemtovgResult<T> = Result<T, femtovg::ErrorKind>;

pub const CLEAR: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

impl Gfx {
    /// Init the graphics infrastructure
    pub fn init(window: Window, font_data: &[u8], font_size: f32) -> Result<Self, String> {
        fn canvas_init(
            window: &Window,
            font_data: &[u8],
            font_size: f32,
        ) -> FemtovgResult<(Canvas, Font)> {
            let renderer =
                unsafe { renderer::OpenGl::new_from_function(|s| window.context_proc_address(s)) }?;

            let mut canvas = Canvas::new(renderer)?;

            let font_id = canvas.add_font_mem(font_data)?;

            let font_height = {
                let mut paint = Paint::default();
                paint.set_font(&[font_id]);
                paint.set_font_size(font_size);
                canvas.measure_font(paint)?.height()
            };

            let font = Font {
                id: font_id,
                size: font_size,
                height: font_height,
            };

            Ok((canvas, font))
        }

        let (canvas, font) = canvas_init(&window, font_data, font_size)
            .map_err(|e| format!("Failed to initialise renderer: {}", e))?;

        Ok(Self {
            canvas,
            font,

            window,
        })
    }

    /// Draw the frame to the window
    /// Should be called after the canvas has been drawn to
    pub fn draw_frame(&mut self) {
        self.canvas.flush();
        self.window.refresh();
    }

    /// Resize the GL context and canvas to fit the window
    pub fn on_resize(&mut self) {
        self.window.resize_context();

        let size = self.window.int_size();
        let dpi_factor = self.window.dpi_factor();
        self.canvas.set_size(size.width, size.height, dpi_factor);
    }
}

impl Font {
    pub fn height(&self) -> f32 {
        self.height
    }
}

/// Canvas utility methods
pub trait CanvasExt {
    fn clear(&mut self);

    fn set_transform_(&mut self, transform: Transform);

    fn set_scissor(&mut self, bounds: Rect);

    fn draw_rect(&mut self, rect: Rect, color: Color);

    fn draw_rect_outline(&mut self, rect: Rect, line_width: f32, color: Color);

    fn draw_image(&mut self, image: ImageId, bounds: Rect);

    fn draw_text(
        &mut self,
        text: &str,
        font: &Font,
        bounds: Rect,
        align: Align,
    ) -> FemtovgResult<f32>;

    fn register_image(
        &mut self,
        image_data: &[u8],
        dimentions: (u32, u32),
    ) -> FemtovgResult<ImageId>;
}

impl CanvasExt for Canvas {
    fn clear(&mut self) {
        self.clear_rect(0, 0, self.width() as u32, self.height() as u32, CLEAR);
    }

    fn set_transform_(&mut self, transform: Transform) {
        let t = transform.to_array();
        self.set_transform(t[0], t[1], t[2], t[3], t[4], t[5]);
    }

    fn set_scissor(&mut self, bounds: Rect) {
        self.scissor(bounds.min.x, bounds.min.y, bounds.width(), bounds.height());
    }

    fn draw_rect(&mut self, rect: Rect, color: Color) {
        let mut path = Path::new();
        path.rect(rect.min.x, rect.min.y, rect.width(), rect.height());

        let paint = Paint::color(color);

        self.fill_path(&mut path, paint);
    }

    fn draw_rect_outline(&mut self, rect: Rect, line_width: f32, color: Color) {
        let mut path = Path::new();
        path.rect(rect.min.x, rect.min.y, rect.width(), rect.height());

        let mut paint = Paint::color(color);
        paint.set_line_width(line_width);

        self.stroke_path(&mut path, paint);
    }

    fn draw_image(&mut self, image: ImageId, bounds: Rect) {
        let mut path = Path::new();
        path.rect(bounds.min.x, bounds.min.y, bounds.width(), bounds.height());

        let paint = Paint::image(
            image,
            bounds.min.x,
            bounds.min.y,
            bounds.width(),
            bounds.height(),
            0.0,
            1.0,
        );

        self.fill_path(&mut path, paint);
    }

    /// Draw text within the bounds with the given align
    /// Returns the width of the final text
    fn draw_text(
        &mut self,
        text: &str,
        font: &Font,
        bounds: Rect,
        align: Align,
    ) -> FemtovgResult<f32> {
        let mut paint = Paint::color(Color::white());
        paint.set_font(&[font.id]);
        paint.set_font_size(font.size);
        paint.set_text_align(align);
        paint.set_text_baseline(Baseline::Top);

        // Find the anchor of the text
        let anchor = {
            let h_span = Vector::new(bounds.width(), 0.0);
            match align {
                Align::Left => bounds.min,
                Align::Center => bounds.min + h_span / 2.0,
                Align::Right => bounds.min + h_span,
            }
        };

        // Restrict drawing to within the bounds
        self.save();
        self.set_scissor(bounds);

        let metrics = self.fill_text(anchor.x, anchor.y, text, paint);

        self.restore();

        metrics.map(|m| m.width())
    }

    /// Register an image into the canvas
    /// * `image_data`: The image data in RGB8 pixels
    fn register_image(
        &mut self,
        image_data: &[u8],
        dimentions: (u32, u32),
    ) -> FemtovgResult<ImageId> {
        let (width, height) = dimentions;

        let pixels = image_data.as_pixels();
        let img = Img::new(pixels, width as usize, height as usize);

        let source = ImageSource::Rgba(img);
        let flags = ImageFlags::empty();

        self.create_image(source, flags)
    }
}
