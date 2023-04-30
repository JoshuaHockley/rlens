//! Module for the lua API

use crate::command::{self, run_command, Command, CommandError};
use crate::input::Key;
use crate::keybinds::KeyBinds;
use crate::program::RequestSender;
use crate::rlens::{Mode, MODES};
use crate::util::StrError;

pub use rlua::prelude::LuaResult;
use rlua::{
    Context, FromLua, FromLuaMulti, Function, RegistryKey, Result, Table, ToLua, ToLuaMulti, Value,
};
use std::fs::read_to_string;
use std::path::Path;
use std::result::Result as StdResult;
use std::sync::{Arc, Mutex};

/// The lua state
pub struct Lua {
    /// Internal lua state
    lua: rlua::Lua,
    /// Registered keybinds
    keybinds: Arc<Mutex<KeyBinds>>,
}

/// The lua API for rlens
/// (See `Lua::context`)
#[derive(Clone, Copy)]
pub struct LuaContext<'lua>(Context<'lua>);

/// A flag that is set as a lua variable before running the rc
#[derive(Clone, Debug)]
pub struct ConfigFlag {
    /// The name of the flag
    pub name: String,
    /// The optional value of the flag
    pub val: Option<String>,
}

/// A key that identifies a specific binding
pub struct BindingKey(RegistryKey);

impl Lua {
    /// Initialise the lua API
    pub fn init(
        request_tx: RequestSender,
        flags: impl IntoIterator<Item = ConfigFlag>,
    ) -> StdResult<Self, String> {
        let lua = Self {
            lua: rlua::Lua::new(),
            keybinds: Arc::new(Mutex::new(KeyBinds::new())),
        };

        lua.context(|ctx| {
            ctx.load_api(request_tx, &lua.keybinds)
                .map_err(|lua_err| format!("Error initialising the lua api: `{}`", lua_err))?;

            // Set config flags
            for flag in flags {
                ctx.set_flag(flag)
                    .map_err(|lua_err| format!("Error setting config flag: {}", lua_err))?;
            }

            Ok::<(), String>(())
        })?;

        Ok(lua)
    }

    /// Perform an action on the lua state
    pub fn context<R, F: FnOnce(LuaContext) -> R>(&self, f: F) -> R {
        self.lua.context(|ctx| f(LuaContext(ctx)))
    }

    /// Try to run a binding for the given key and mode
    /// No effect if a binding is not present
    pub fn try_keybind(&self, key: &Key, mode: Mode) -> Result<()> {
        let keybinds = self.keybinds.lock().unwrap();

        self.context(|ctx| {
            // Check if a binding is present
            if let Some(binding_key) = keybinds.lookup_key(key, mode) {
                assert!(ctx.0.owns_registry_value(&binding_key.0));

                // Lookup the binding in the registry
                let binding: Function = ctx
                    .0
                    .registry_value(&binding_key.0)
                    .expect("Binding keys must point to Functions");

                // Release the lock on keybinds so our binding can create bindings
                drop(keybinds);

                // Call the bound function
                let _: Value = binding.call(())?;
            }

            Ok(())
        })
    }

    /// Run the lua RC at `rc_path`
    pub fn run_rc(&self, rc_path: &Path) -> StdResult<(), String> {
        let rc = read_to_string(&rc_path)
            .map_err(|e| format!("Error loading rc at `{}`: {}", rc_path.display(), e))?;
        self.context(|ctx| ctx.run(&rc))
            .map_err(|e| format!("Error running rc: {}", e))
    }
}

impl<'lua> LuaContext<'lua> {
    /// Run arbitrary code in lua
    fn run(&self, code: &str) -> Result<()> {
        self.0.load(code).exec()
    }

    /// Evaluate arbitrary code in lua and try to coerce the result to a string
    /// `Err(_)` if an error is raised by the lua code
    /// `Ok(None)` if the code evaluated successfully, but could not be coerced to a `String`
    fn _eval(&self, code: &str) -> Result<Option<String>> {
        // Evaluate the code
        let v = self.0.load(code).eval::<Value>()?;

        // Coerce to string
        Ok(String::from_lua(v, self.0).ok())
    }
}

// API table names
const API_TABLES: &[&str] = &[RLENS_TABLE, HOOK_TABLE, QUERY_TABLE, FLAG_TABLE];
const RLENS_TABLE: &str = "rlens";
const HOOK_TABLE: &str = "hook";
const QUERY_TABLE: &str = "query";
const FLAG_TABLE: &str = "flag";

impl<'lua> LuaContext<'lua> {
    /// Load the rlens API functions and prepare the API tables
    fn load_api(&self, tx: RequestSender, keybinds: &Arc<Mutex<KeyBinds>>) -> Result<()> {
        self.init_tables()?;

        self.load_global("bind", bind_all(keybinds.clone()))?;
        self.load_global("bind_image", bind_mode(Mode::Image, keybinds.clone()))?;
        self.load_global("bind_gallery", bind_mode(Mode::Gallery, keybinds.clone()))?;

        self.load_rlens("exit", wrap_nullary_command(|| command::Exit, &tx))?;

        self.load_rlens("mode", wrap_command(command::Mode, &tx))?;
        self.load_rlens(
            "current_mode",
            wrap_nullary_command(|| command::CurrentMode, &tx),
        )?;

        self.load_rlens("select", wrap_nullary_command(|| command::Select, &tx))?;

        self.load_rlens("index", wrap_nullary_command(|| command::Index, &tx))?;
        self.load_rlens(
            "total_images",
            wrap_nullary_command(|| command::TotalImages, &tx),
        )?;

        self.load_rlens("image", wrap_command(command::Image, &tx))?;
        self.load_rlens(
            "current_image",
            wrap_nullary_command(|| command::CurrentImage, &tx),
        )?;

        self.load_rlens("goto", wrap_command(command::Goto, &tx))?;
        self.load_rlens("next", wrap_nullary_command(|| command::Next, &tx))?;
        self.load_rlens(
            "next_wrapping",
            wrap_nullary_command(|| command::NextWrapping, &tx),
        )?;
        self.load_rlens("prev", wrap_nullary_command(|| command::Prev, &tx))?;
        self.load_rlens(
            "prev_wrapping",
            wrap_nullary_command(|| command::PrevWrapping, &tx),
        )?;
        self.load_rlens("first", wrap_nullary_command(|| command::First, &tx))?;
        self.load_rlens("last", wrap_nullary_command(|| command::Last, &tx))?;

        self.load_rlens("gallery_goto", wrap_command(command::GalleryGoto, &tx))?;
        self.load_rlens(
            "gallery_next",
            wrap_nullary_command(|| command::GalleryNext, &tx),
        )?;
        self.load_rlens(
            "gallery_next_wrapping",
            wrap_nullary_command(|| command::GalleryNextWrapping, &tx),
        )?;
        self.load_rlens(
            "gallery_prev",
            wrap_nullary_command(|| command::GalleryPrev, &tx),
        )?;
        self.load_rlens(
            "gallery_prev_wrapping",
            wrap_nullary_command(|| command::GalleryPrevWrapping, &tx),
        )?;
        self.load_rlens(
            "gallery_first",
            wrap_nullary_command(|| command::GalleryFirst, &tx),
        )?;
        self.load_rlens(
            "gallery_last",
            wrap_nullary_command(|| command::GalleryLast, &tx),
        )?;
        self.load_rlens(
            "gallery_up",
            wrap_nullary_command(|| command::GalleryUp, &tx),
        )?;
        self.load_rlens(
            "gallery_down",
            wrap_nullary_command(|| command::GalleryDown, &tx),
        )?;

        self.load_rlens("reset", wrap_nullary_command(|| command::Reset, &tx))?;

        self.load_rlens("pan", wrap_command(|(dx, dy)| command::Pan(dx, dy), &tx))?;
        self.load_rlens("zoom", wrap_command(command::Zoom, &tx))?;
        self.load_rlens("rotate", wrap_command(command::Rotate, &tx))?;
        self.load_rlens("hflip", wrap_nullary_command(|| command::HFlip, &tx))?;
        self.load_rlens("vflip", wrap_nullary_command(|| command::VFlip, &tx))?;

        self.load_rlens(
            "set_pan",
            wrap_command(|(dx, dy)| command::SetPan(dx, dy), &tx),
        )?;
        self.load_rlens("set_zoom", wrap_command(command::SetZoom, &tx))?;
        self.load_rlens("set_rotation", wrap_command(command::SetRotation, &tx))?;
        self.load_rlens("set_flipped", wrap_command(command::SetFlipped, &tx))?;

        self.load_rlens("scaling", wrap_command(command::Scaling, &tx))?;

        self.load_rlens("align_x", wrap_command(command::AlignX, &tx))?;
        self.load_rlens("align_y", wrap_command(command::AlignY, &tx))?;

        self.load_rlens(
            "transform",
            wrap_nullary_command(|| command::Transform, &tx),
        )?;

        self.load_rlens("reload", wrap_nullary_command(|| command::Reload, &tx))?;

        self.load_rlens(
            "preload_range",
            wrap_command(
                |(forward, backward)| command::PreloadRange(forward, backward),
                &tx,
            ),
        )?;

        self.load_rlens(
            "save_thumbnails",
            wrap_command(command::SaveThumbnails, &tx),
        )?;

        self.load_rlens(
            "gallery_tile_width",
            wrap_command(command::GalleryTileWidth, &tx),
        )?;
        self.load_rlens(
            "gallery_height_width_ratio",
            wrap_command(command::GalleryHeightWidthRatio, &tx),
        )?;

        self.load_rlens("status_bar", wrap_command(command::StatusBar, &tx))?;
        self.load_rlens(
            "toggle_status_bar",
            wrap_nullary_command(|| command::ToggleStatusBar, &tx),
        )?;
        self.load_rlens(
            "refresh_status_bar",
            wrap_nullary_command(|| command::RefreshStatusBar, &tx),
        )?;
        self.load_rlens(
            "status_bar_position",
            wrap_command(command::StatusBarPosition, &tx),
        )?;

        self.load_rlens("fullscreen", wrap_command(command::FullScreen, &tx))?;
        self.load_rlens(
            "toggle_fullscreen",
            wrap_nullary_command(|| command::ToggleFullScreen, &tx),
        )?;

        self.load_rlens("freeze", wrap_nullary_command(|| command::Freeze, &tx))?;
        self.load_rlens("unfreeze", wrap_nullary_command(|| command::Unfreeze, &tx))?;

        self.load_rlens("bg_color", wrap_command(command::BgColor, &tx))?;
        self.load_rlens("backdrop_color", wrap_command(command::BackdropColor, &tx))?;
        self.load_rlens(
            "gallery_cursor_color",
            wrap_command(command::GalleryCursorColor, &tx),
        )?;
        self.load_rlens(
            "gallery_border_color",
            wrap_command(command::GalleryBorderColor, &tx),
        )?;
        self.load_rlens(
            "status_bar_color",
            wrap_command(command::StatusBarColor, &tx),
        )?;

        Ok(())
    }

    /// Create the API tables in the global scope
    fn init_tables(&self) -> Result<()> {
        let create_table = |ident: &str| {
            let t = self.0.create_table()?;
            self.0.globals().set(ident, t)
        };

        for ident in API_TABLES {
            create_table(ident)?;
        }

        Ok(())
    }

    /// Load a function into a scope
    fn load_function<A, R, F>(&self, ident: &str, scope: Table<'lua>, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLuaMulti<'lua>,
        F: 'static + Send + Fn(Context<'lua>, A) -> Result<R>,
    {
        let f = self.0.create_function(func)?;
        scope.set(ident, f)
    }

    /// Load a function into the global scope
    fn load_global<A, R, F>(&self, ident: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLuaMulti<'lua>,
        F: 'static + Send + Fn(Context<'lua>, A) -> Result<R>,
    {
        let scope = self.0.globals();
        self.load_function(ident, scope, func)
    }

    /// Load a function into the `rlens` table
    /// Pre: The `rlens` table has been created (see `init_tables`)
    fn load_rlens<A, R, F>(&self, ident: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLuaMulti<'lua>,
        F: 'static + Send + Fn(Context<'lua>, A) -> Result<R>,
    {
        let scope = self.0.globals().get(RLENS_TABLE)?;
        self.load_function(ident, scope, func)
    }

    /// Set a config flag
    /// Pre: The `flag` table has been created (see `init_tables`)
    fn set_flag(&self, flag: ConfigFlag) -> Result<()> {
        // Convert the flag's value to the appropriate lua value
        let lua_val = match flag.val {
            Some(s) => s.to_lua(self.0)?,
            None => Value::Boolean(true),
        };

        // Set the flag in the flag table
        let scope: Table = self.0.globals().get(FLAG_TABLE)?;
        scope.set(flag.name, lua_val)?;

        Ok(())
    }
}

/// Wrap a command in a lua function
/// `cmd_f` bridges the gap between the lua arguments and the `Command`
fn wrap_command<A, C: Command>(
    cmd_f: impl Fn(A) -> C,
    request_tx: &RequestSender,
) -> impl Fn(Context, A) -> Result<C::Output> {
    let tx = request_tx.clone();

    move |ctx, args| {
        let cmd = cmd_f(args);
        let out = run_command(cmd, &tx, LuaContext(ctx))?;
        Ok(out)
    }
}

/// Wrap a command in a nullary lua function (convenient shortcut over `wrap_command`)
fn wrap_nullary_command<C: Command>(
    cmd: impl Fn() -> C,
    request_tx: &RequestSender,
) -> impl Fn(Context, ()) -> Result<C::Output> {
    let cmd_f = move |()| cmd();

    wrap_command(cmd_f, request_tx)
}

/// Bind a key and mode to a function
fn bind<'lua>(
    key_str: &str,
    mode: Mode,
    binding: Function<'lua>,
    keybinds: &Mutex<KeyBinds>,
    ctx: Context<'lua>,
) -> Result<()> {
    let key = key_str
        .parse()
        .map_err(|_| StrError(format!("Unrecognised key identifier: `{}`", key_str)))?;

    // Put the function into the registry
    let binding_key = BindingKey(ctx.create_registry_value(binding)?);

    // Set the keybind
    keybinds.lock().unwrap().update(key, mode, binding_key);

    // Remove any replaced bindings
    ctx.expire_registry_values();

    Ok(())
}

/// lua callback to bind a key and mode to a function
fn bind_mode(
    mode: Mode,
    keybinds: Arc<Mutex<KeyBinds>>,
) -> impl for<'lua> Fn(Context<'lua>, (String, Function<'lua>)) -> Result<()> {
    move |ctx, (key_str, binding)| bind(&key_str, mode, binding, &keybinds, ctx)
}

/// lua callback to bind a key to a function for all modes
fn bind_all(
    keybinds: Arc<Mutex<KeyBinds>>,
) -> impl for<'lua> Fn(Context<'lua>, (String, Function<'lua>)) -> Result<()> {
    move |ctx, (key_str, binding)| {
        for &mode in MODES {
            bind(&key_str, mode, binding.clone(), &keybinds, ctx)?;
        }
        Ok(())
    }
}

impl<'lua> LuaContext<'lua> {
    /// Try to call a nullary function by name
    /// Returns `None` if the function was not found in the scope
    fn call_function<T: FromLuaMulti<'lua>>(
        &self,
        function_ident: &str,
        scope: Table<'lua>,
    ) -> Result<Option<T>> {
        let f: Option<Function> = scope.get(function_ident).ok();
        f.map(|f| f.call(())).transpose()
    }

    /// Call a hook by name
    pub fn call_hook(&self, hook: &str) -> Result<()> {
        if let Ok(scope) = self.0.globals().get(HOOK_TABLE) {
            self.call_function::<Value>(hook, scope).map(|_| ())
        } else {
            Ok(())
        }
    }

    /// Call a query by name
    pub fn call_query<T: FromLuaMulti<'lua>>(&self, query: &str) -> Result<Option<T>> {
        if let Ok(scope) = self.0.globals().get(QUERY_TABLE) {
            self.call_function(query, scope)
        } else {
            Ok(None)
        }
    }
}

// Wrap `CmdError` in `rlua::Error`
impl From<CommandError> for rlua::Error {
    fn from(cmd_err: CommandError) -> rlua::Error {
        rlua::Error::external(cmd_err)
    }
}
