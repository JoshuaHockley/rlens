//! Module for managing modal keybinds

use crate::input::Key;
use crate::lua::BindingKey;
use crate::rlens::Mode;

use enum_map::EnumMap;
use std::collections::HashMap;

/// The mapping from keys to their modal bindings
pub struct KeyBinds(HashMap<Key, ModeMap>);

/// A mapping from modes to potential keybinds
type ModeMap = EnumMap<Mode, Option<BindingKey>>;

impl KeyBinds {
    /// Empty keybinds
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Lookup the binding for a key in the given mode
    pub fn lookup_key(&self, key: &Key, mode: Mode) -> Option<&BindingKey> {
        self.0.get(key).and_then(|mode_map| mode_map[mode].as_ref())
    }

    /// Update a keybind
    /// Replaced `BindingKey`s are dropped
    pub fn update(&mut self, key: Key, mode: Mode, binding_key: BindingKey) {
        let mode_map = self.0.entry(key).or_insert(ModeMap::default());
        mode_map[mode] = Some(binding_key);
    }
}
