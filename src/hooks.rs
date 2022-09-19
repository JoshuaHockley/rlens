//! Module for lua hooks

use crate::lua::LuaContext;
use crate::util::PrintLuaErr;

/// lua hooks triggered by commands
#[derive(Default)]
pub struct Hooks {
    current_image_change: bool,
    transform_update: bool,
}

impl Hooks {
    /// Run the hooks
    pub fn run(&self, lua_ctx: LuaContext) {
        for (flag, name) in [
            (self.current_image_change, "current_image_change"),
            (self.transform_update, "transform_update"),
        ] {
            if flag {
                lua_ctx.call_hook(name).print_lua_err().ok();
            }
        }
    }

    pub fn current_image_change(&mut self) {
        self.current_image_change = true;
    }
    pub fn transform_update(&mut self) {
        self.transform_update = true;
    }
}

/// A hook that is triggered by an external event
pub enum ExternalHook {
    /// The current image was loaded
    CurrentImageLoad,
    /// The window was resized
    WindowResize,
}

impl ExternalHook {
    /// Run the hook
    pub fn run(&self, lua_ctx: LuaContext) {
        let name = self.name();
        lua_ctx.call_hook(name).print_lua_err().ok();
    }

    /// The name of the hook under lua
    fn name(&self) -> &'static str {
        match self {
            Self::CurrentImageLoad => "current_image_load",
            Self::WindowResize => "resize",
        }
    }
}
