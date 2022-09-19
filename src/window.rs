//! Module for creating and managing the window and GL context

use crate::geometry::*;
use crate::program::EventLoop;
use crate::util::PrintErr;

use glutin::window::{self, WindowBuilder};
use glutin::{
    event_loop::EventLoopBuilder, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext,
};
use std::ffi::c_void;

/// Handle to the window and GL context
pub struct Window(WindowedContext<PossiblyCurrent>);

impl Window {
    /// Create the window, GL context, and event loop
    pub fn create() -> Result<(Self, EventLoop), String> {
        let event_loop = EventLoopBuilder::with_user_event().build();
        let window_builder = WindowBuilder::new().with_title("rlens");

        // Build the window with a GL context
        let windowed_context = ContextBuilder::new()
            .with_gl(GlRequest::Latest)
            .build_windowed(window_builder, &event_loop)
            .map_err(|e| format!("Failed to create window: {}", e))?;

        // Make the context current
        let windowed_context = unsafe { windowed_context.make_current() }
            .map_err(|(_, e)| format!("Failed to make the GL context current: {}", e))?;

        Ok((Self(windowed_context), event_loop))
    }

    /// Get the proc address of the context
    pub fn context_proc_address(&self, addr: &str) -> *const c_void {
        self.0.get_proc_address(addr)
    }

    /// Refresh the window content
    pub fn refresh(&self) {
        self.0.swap_buffers().print_err().ok();
    }

    /// Resize the GL context to fit the window
    /// Should be called after window resizes
    pub fn resize_context(&self) {
        self.0.resize(self.0.window().inner_size());
    }

    pub fn int_size(&self) -> IntSize {
        let size = self.0.window().inner_size();
        IntSize::new(size.width, size.height)
    }

    pub fn size(&self) -> Size {
        self.int_size().to_f32()
    }

    pub fn dpi_factor(&self) -> f32 {
        self.0.window().scale_factor() as f32
    }

    /// Check if the window is fullscreen
    pub fn is_fullscreen(&self) -> bool {
        self.0.window().fullscreen().is_some()
    }

    /// Set whether the window is fullscreen
    pub fn set_fullscreen(&self, on: bool) {
        let new_mode = on.then_some(window::Fullscreen::Borderless(None));
        self.0.window().set_fullscreen(new_mode)
    }
}
