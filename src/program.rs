//! Module for overall program structure and the event loop

use crate::command::CommandRequestT;
use crate::geometry::Size;
use crate::gfx::Gfx;
use crate::hooks::ExternalHook;
use crate::image_loader::run_image_loader;
use crate::input::Key;
use crate::load_request::{LoadRequest, LoadRequestResponse};
use crate::lua::{ConfigFlag, Lua};
use crate::rlens::{Mode, RLens};
use crate::util::{PrintErr, PrintLuaErr};
use crate::window::Window;

use glutin::{
    event::{self, KeyboardInput},
    event_loop,
    platform::run_return::EventLoopExtRunReturn,
};
use std::borrow::Cow;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender, SyncSender};
use std::thread::{spawn, JoinHandle};

/// Run rlens and exit safely
pub fn rlens(images: Vec<PathBuf>, initial_index: usize, settings: Settings) -> Result<(), String> {
    let (mut program, event_loop) = Program::init(images, initial_index, settings)?;

    program.run(event_loop);

    program.shutdown();

    Ok(())
}

/// Components and state of the program
pub struct Program {
    /// The state of rlens
    pub rlens: RLens,

    /// Graphics API
    pub gfx: Gfx,

    /// Exit flag
    pub exit: bool,

    /// Sender for lua requests
    lua_request_tx: Sender<LuaRequest>,
    /// Handle to the lua thread
    lua_thread: JoinHandle<()>,

    /// Sender for load requests
    /// Blocks until the request is retrieved by the image loader thread
    load_request_tx: SyncSender<LoadRequest>,
    /// Flag for whether the image loader is currently waiting for a load request
    image_loader_waiting: bool,
    /// Handle to the image loader thread
    image_loader_thread: JoinHandle<()>,
}

/// Settings provided on startup
#[derive(Debug)]
pub struct Settings {
    /// Path to the lua rc
    pub rc_path: PathBuf,
    /// Config flags
    pub config_flags: Vec<ConfigFlag>,
    /// Path of the thumbnail directory
    pub thumbnail_dir: PathBuf,
    /// Size to generate thumbnails at
    /// (fit within 'size x size')
    pub thumbnail_size: u32,
    /// Raw font data from a ttf/otf
    pub font_data: Cow<'static, [u8]>,
    /// Font size in pixels
    pub font_size: f32,
}

impl Program {
    fn init(
        images: Vec<PathBuf>,
        initial_index: usize,
        settings: Settings,
    ) -> Result<(Self, EventLoop), String> {
        let (window, event_loop) = Window::create()?;

        let request_tx = RequestSender::new(event_loop.create_proxy());

        let gfx = Gfx::init(window, &settings.font_data, settings.font_size)?;

        let rlens = RLens::init(images, initial_index);

        let lua = Lua::init(request_tx.clone(), settings.config_flags)?;

        let (lua_request_tx, lua_thread) = run_lua_thread(lua);
        lua_request_tx
            .send(LuaRequest::RunRC(settings.rc_path))
            .unwrap();

        let (load_request_tx, image_loader_thread) = run_image_loader(
            request_tx.clone(),
            settings.thumbnail_dir,
            settings.thumbnail_size,
        );

        let program = Self {
            rlens,

            gfx,

            lua_request_tx,
            lua_thread,

            load_request_tx,
            image_loader_waiting: false,
            image_loader_thread,

            exit: false,
        };

        Ok((program, event_loop))
    }

    /// Run the event loop on the main thread and handle `Request`s
    /// Returns when the exit flag is set, closing the request channel
    fn run(&mut self, mut event_loop: EventLoop) {
        event_loop.run_return(|event, _, control_flow| {
            self.handle_event(event);

            *control_flow = if self.exit {
                event_loop::ControlFlow::Exit
            } else {
                event_loop::ControlFlow::Wait
            };
        });
    }

    /// Handle an event or request
    fn handle_event(&mut self, event: Event) {
        use event::{Event::*, WindowEvent::*};

        match event {
            UserEvent(req) => {
                self.handle_request(req);
            }
            WindowEvent { event, .. } => match event {
                CloseRequested => {
                    self.exit = true;
                }
                Resized(_) => {
                    self.on_resize();
                }
                KeyboardInput { input, .. } => {
                    self.handle_key(input);
                }
                _ => {}
            },
            RedrawRequested(_) => {
                self.draw();
            }
            _ => {}
        }
    }

    /// Respond to keyboard input
    fn handle_key(&self, key_event: KeyboardInput) {
        if let Ok(key) = key_event.try_into() {
            let lua_req = LuaRequest::Keybind(key, self.rlens.mode());
            self.lua_request_tx.send(lua_req).unwrap();
        }
    }

    /// Respond to a resizing of the window
    fn on_resize(&mut self) {
        // Update graphics infrastructure
        self.gfx.on_resize();

        // Thumbnail loading
        if self.rlens.mode() == Mode::Gallery {
            self.wake_image_loader();
        }

        // Call lua hook
        self.lua_request_tx
            .send(LuaRequest::Hook(ExternalHook::WindowResize))
            .unwrap();
    }

    /// Safely shutdown the program
    fn shutdown(self) {
        let Self {
            lua_request_tx,
            lua_thread,

            load_request_tx,
            image_loader_thread,
            ..
        } = self;

        drop(lua_request_tx);
        lua_thread.join().unwrap();

        drop(load_request_tx);
        image_loader_thread.join().unwrap();
    }
}

impl Program {
    /// Draw the state of the program
    pub fn draw(&mut self) {
        let view = self.window_size();
        self.rlens.draw(&mut self.gfx, view);
    }

    /// Get the current size of the window
    pub fn window_size(&self) -> Size {
        self.gfx.window.size()
    }

    /// Wake the image loader thread with a new load request if possible
    /// This should be called when the result of `Rlens::poll_loads` may have changed
    /// (e.g. changed current image)
    ///
    /// See `Request::ImageLoaderReady`
    ///
    pub fn wake_image_loader(&mut self) {
        if self.image_loader_waiting {
            // The image loader thread is currently waiting for a load request,
            // so we can send one without blocking
            if let Some(req) = self.rlens.poll_loads(self.window_size(), &self.gfx.font) {
                // Send the load request
                self.load_request_tx.send(req).unwrap();

                // Reset the flag
                self.image_loader_waiting = false;
            }
        } else {
            // The image loader is busy, and will notify us when it is ready for a load request
        }
    }
}

/// A request to the main thread
#[derive(Debug)]
pub enum Request {
    /// Run a command's main body
    CommandRequest(Box<dyn CommandRequestT>),

    /// The image loader is ready for a load request
    /// This is made by the image loader thread immediately before waiting for a request
    ImageLoaderReady,
    /// Load an image from the raw data
    LoadImage(LoadRequestResponse),
    /// Mark an image's source as unloadable
    MarkUnloadable(usize),
    /// Unload any out-of-range images
    UnloadImages,
}

type Event<'a> = event::Event<'a, Request>;
pub type EventLoop = event_loop::EventLoop<Request>;
type EventLoopProxy = event_loop::EventLoopProxy<Request>;
type EventLoopClosed = event_loop::EventLoopClosed<Request>;

impl Program {
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::CommandRequest(cmd_req) => {
                cmd_req.handle(self);
            }

            Request::ImageLoaderReady => {
                if let Some(req) = self.rlens.poll_loads(self.window_size(), &self.gfx.font) {
                    // Send the load request
                    self.load_request_tx.send(req).unwrap();
                } else {
                    // We have no immediate need for the image loader, so let it sleep
                    // Set the flag so we know to respond later
                    // See `Program::wake_image_loader`
                    self.image_loader_waiting = true;
                }
            }
            Request::LoadImage(LoadRequestResponse {
                type_,
                index,
                image,
                metadata,
            }) => {
                // Load the image into the canvas
                let loaded = match image.load_into_canvas(&mut self.gfx).print_err() {
                    Ok(loaded) => loaded,
                    _ => {
                        return;
                    }
                };

                // Update the image list
                let (redraw, current_load) =
                    self.rlens
                        .set_loaded(type_, index, loaded, metadata, self.window_size());

                if redraw {
                    self.draw();
                }
                if current_load {
                    self.lua_request_tx
                        .send(LuaRequest::Hook(ExternalHook::CurrentImageLoad))
                        .unwrap();
                }
            }
            Request::MarkUnloadable(index) => {
                self.rlens.mark_unloadable(index);
            }
            Request::UnloadImages => {
                self.rlens.unload_images(&mut self.gfx);
            }
        }
    }
}

/// Run the lua thread
/// Returns after the sender is dropped
fn run_lua_thread(lua: Lua) -> (Sender<LuaRequest>, JoinHandle<()>) {
    // Create the request channel
    let (sender, receiver) = channel::<LuaRequest>();

    let thread = spawn(move || {
        // Loop until the channel is closed
        while let Ok(req) = receiver.recv() {
            // Handle the request
            req.handle(&lua);
        }
    });

    (sender, thread)
}

/// A request to the lua thread
enum LuaRequest {
    /// Try the keybind for the given key in the given mode
    Keybind(Key, Mode),
    /// Run a hook
    Hook(ExternalHook),
    /// Run the RC
    RunRC(PathBuf),
}

impl LuaRequest {
    /// Handle a request on the lua thread
    fn handle(&self, lua: &Lua) {
        match self {
            Self::Keybind(key, mode) => {
                lua.try_keybind(key, *mode).print_lua_err().ok();
            }
            Self::Hook(hook) => {
                lua.context(|ctx| hook.run(ctx));
            }
            Self::RunRC(rc_path) => {
                lua.run_rc(&rc_path).print_err().ok();
            }
        }
    }
}

/// Handle for sending rlens requests to the main thread
#[derive(Clone)]
pub struct RequestSender(EventLoopProxy);

impl RequestSender {
    pub fn new(events_proxy: EventLoopProxy) -> Self {
        Self(events_proxy)
    }

    /// Send a request
    pub fn send(&self, req: Request) -> Result<(), EventLoopClosed> {
        self.0.send_event(req)
    }
}
