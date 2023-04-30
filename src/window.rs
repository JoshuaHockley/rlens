//! Module for creating and managing the window and GL context

use crate::geometry::*;
use crate::program::EventLoop;
use crate::util::PrintErr;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use std::ffi::{c_void, CStr};
use std::num::NonZeroU32;
use winit::{
    event_loop::EventLoopBuilder,
    window::{Fullscreen, WindowBuilder},
};

/// Handle to the window and GL context
pub struct Window {
    window: winit::window::Window,

    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
}

impl Window {
    /// Create the window, GL context, and event loop
    pub fn create() -> Result<(Self, EventLoop), String> {
        let event_loop = EventLoopBuilder::with_user_event().build();

        let window_builder = WindowBuilder::new()
            .with_title("rlens")
            .with_transparent(true);

        let (window, config) = DisplayBuilder::new()
            .with_window_builder(Some(window_builder))
            .build(&event_loop, ConfigTemplateBuilder::new(), |configs| {
                configs
                    .max_by_key(|c| c.num_samples())
                    .expect("No GL configs were found")
            })
            .map_err(|e| format!("Failed to create window: {}", e))?;
        let window = window.unwrap(); // Safe, as we passed a window builder

        let display = config.display();

        let surface = {
            let attrs = window.build_surface_attributes(<_>::default());
            unsafe { display.create_window_surface(&config, &attrs) }
                .map_err(|e| format!("Failed to create gl surface: {}", e))?
        };

        let context = {
            let attrs = ContextAttributesBuilder::new().build(Some(window.raw_window_handle()));
            unsafe { display.create_context(&config, &attrs) }
                .map_err(|e| format!("Failed to create gl context: {}", e))?
                .make_current(&surface)
                .map_err(|e| format!("Failed to make the gl context current: {}", e))?
        };

        Ok((
            Self {
                window,
                surface,
                context,
            },
            event_loop,
        ))
    }

    /// Get the proc address of the context
    pub fn context_proc_address(&self, addr: &CStr) -> *const c_void {
        self.context.display().get_proc_address(addr)
    }

    /// Refresh the window content
    pub fn refresh(&self) {
        self.surface.swap_buffers(&self.context).print_err().ok();
    }

    /// Resize the GL context to fit the window
    /// Should be called after window resizes
    pub fn resize_context(&self) {
        let size = self.window.inner_size();
        if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
            self.surface.resize(&self.context, w, h);
        }
    }

    pub fn int_size(&self) -> IntSize {
        let size = self.window.inner_size();
        IntSize::new(size.width, size.height)
    }

    pub fn size(&self) -> Size {
        self.int_size().to_f32()
    }

    pub fn dpi_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    /// Check if the window is fullscreen
    pub fn is_fullscreen(&self) -> bool {
        self.window.fullscreen().is_some()
    }

    /// Set whether the window is fullscreen
    pub fn set_fullscreen(&self, on: bool) {
        let new_mode = on.then_some(Fullscreen::Borderless(None));
        self.window.set_fullscreen(new_mode)
    }
}
