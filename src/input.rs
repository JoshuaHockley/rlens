//! Module for handling keyboard input for keybindings

use std::str::FromStr;
use winit::event::{ElementState, KeyboardInput, ModifiersState, VirtualKeyCode};

/// Identification for a specific keypress, including modifiers
/// The type is opaque and should only be used for its `Eq` and `Hash` capabilities
//
//  This is implemented as a wrapper around relevant parts of `winit::event::KeyboardInput`
//
#[derive(PartialEq, Eq, Hash)]
pub struct Key {
    virtual_keycode: VirtualKeyCode,
    modifiers: ModifiersState,
}

impl TryFrom<KeyboardInput> for Key {
    type Error = ();

    fn try_from(kb_input: KeyboardInput) -> Result<Self, Self::Error> {
        #![allow(deprecated)]
        let KeyboardInput {
            state,
            virtual_keycode,
            modifiers,
            ..
        } = kb_input;

        // Ignore key releases
        match state {
            ElementState::Pressed => Ok(()),
            _ => Err(()),
        }?;

        let virtual_keycode = virtual_keycode.ok_or(())?;

        Ok(Self {
            virtual_keycode,
            modifiers,
        })
    }
}

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        key_identifier(s).ok_or(())
    }
}

/// Parse a key identifier
///
/// Format:
///     [S-][C-][A-][L-]<keycode>
/// S: Shift
/// C: Ctrl
/// A: Alt
/// L: Logo / Command / Super
///
/// The keycode refers to the true character being inputted, taking the shift key into account.
/// For example, 'shift-8' sends the `*` character, not `8`.
/// An exception of this is the letters keys A-Z. They are only accepted in lowercase form (`a`).
///
/// If the keypress involves the shift key, the `S` marker must be present, even if the keycode
/// implies the shifting itself.
/// For example, an input of `*` can only be matched as `S-*`, and not `*` alone.
///
fn key_identifier(s: &str) -> Option<Key> {
    // Handle modifiers
    let mut s = s;
    let mut modifiers = ModifiersState::empty();
    if strip_modifier(&mut s, "S") {
        modifiers |= ModifiersState::SHIFT;
    }
    if strip_modifier(&mut s, "C") {
        modifiers |= ModifiersState::CTRL
    }
    if strip_modifier(&mut s, "A") {
        modifiers |= ModifiersState::ALT
    }
    if strip_modifier(&mut s, "L") {
        modifiers |= ModifiersState::LOGO
    }

    // Parse keycode
    let virtual_keycode = parse_keycode(s)?;

    Some(Key {
        virtual_keycode,
        modifiers,
    })
}

/// Try to strip the modifier prefix from `s`
/// `s`: "C-f", `modifier`: "C"
/// -> `s`: "f", true
fn strip_modifier(s: &mut &str, modifier: &str) -> bool {
    const SEPERATOR_CHAR: char = '-';
    if let Some(rem) = s
        .strip_prefix(modifier)
        .and_then(|s| s.strip_prefix(SEPERATOR_CHAR))
    {
        *s = rem;
        true
    } else {
        false
    }
}

fn parse_keycode(s: &str) -> Option<VirtualKeyCode> {
    use VirtualKeyCode::*;

    match s {
        "1" => Ok(Key1),
        "2" => Ok(Key2),
        "3" => Ok(Key3),
        "4" => Ok(Key4),
        "5" => Ok(Key5),
        "6" => Ok(Key6),
        "7" => Ok(Key7),
        "8" => Ok(Key8),
        "9" => Ok(Key9),
        "0" => Ok(Key0),

        "a" | "A" => Ok(A),
        "b" | "B" => Ok(B),
        "c" | "C" => Ok(C),
        "d" | "D" => Ok(D),
        "e" | "E" => Ok(E),
        "f" | "F" => Ok(F),
        "g" | "G" => Ok(G),
        "h" | "H" => Ok(H),
        "i" | "I" => Ok(I),
        "j" | "J" => Ok(J),
        "k" | "K" => Ok(K),
        "l" | "L" => Ok(L),
        "m" | "M" => Ok(M),
        "n" | "N" => Ok(N),
        "o" | "O" => Ok(O),
        "p" | "P" => Ok(P),
        "q" | "Q" => Ok(Q),
        "r" | "R" => Ok(R),
        "s" | "S" => Ok(S),
        "t" | "T" => Ok(T),
        "u" | "U" => Ok(U),
        "v" | "V" => Ok(V),
        "w" | "W" => Ok(W),
        "x" | "X" => Ok(X),
        "y" | "Y" => Ok(Y),
        "z" | "Z" => Ok(Z),

        "Esc" => Ok(Escape),

        "F1" => Ok(F1),
        "F2" => Ok(F2),
        "F3" => Ok(F3),
        "F4" => Ok(F4),
        "F5" => Ok(F5),
        "F6" => Ok(F6),
        "F7" => Ok(F7),
        "F8" => Ok(F8),
        "F9" => Ok(F9),
        "F10" => Ok(F10),
        "F11" => Ok(F11),
        "F12" => Ok(F12),
        "F13" => Ok(F13),
        "F14" => Ok(F14),
        "F15" => Ok(F15),
        "F16" => Ok(F16),
        "F17" => Ok(F17),
        "F18" => Ok(F18),
        "F19" => Ok(F19),
        "F20" => Ok(F20),
        "F21" => Ok(F21),
        "F22" => Ok(F22),
        "F23" => Ok(F23),
        "F24" => Ok(F24),

        "Snapshot" => Ok(Snapshot),
        "Scroll" => Ok(Scroll),
        "Pause" => Ok(Pause),
        "Insert" => Ok(Insert),
        "Home" => Ok(Home),
        "Delete" => Ok(Delete),
        "End" => Ok(End),
        "PageDown" => Ok(PageDown),
        "PageUp" => Ok(PageUp),

        "Left" => Ok(Left),
        "Right" => Ok(Right),
        "Up" => Ok(Up),
        "Down" => Ok(Down),

        "Back" => Ok(Back),
        "Return" => Ok(Return),
        "Space" => Ok(Space),
        "Compose" => Ok(Compose),

        "Caret" | "^" => Ok(Caret),

        "Numlock" => Ok(Numlock),

        "Numpad0" => Ok(Numpad0),
        "Numpad1" => Ok(Numpad1),
        "Numpad2" => Ok(Numpad2),
        "Numpad3" => Ok(Numpad3),
        "Numpad4" => Ok(Numpad4),
        "Numpad5" => Ok(Numpad5),
        "Numpad6" => Ok(Numpad6),
        "Numpad7" => Ok(Numpad7),
        "Numpad8" => Ok(Numpad8),
        "Numpad9" => Ok(Numpad9),

        "NumpadAdd" => Ok(NumpadAdd),
        "NumpadDivide" => Ok(NumpadDivide),
        "NumpadDecimal" => Ok(NumpadDecimal),
        "NumpadComma" => Ok(NumpadComma),
        "NumpadEnter" => Ok(NumpadEnter),
        "NumpadEquals" => Ok(NumpadEquals),
        "NumpadMultiply" => Ok(NumpadMultiply),
        "NumpadSubtract" => Ok(NumpadSubtract),

        "AbntC1" => Ok(AbntC1),
        "AbntC2" => Ok(AbntC2),
        "Apostrophe" | "'" => Ok(Apostrophe),
        "Apps" => Ok(Apps),
        "Asterisk" | "*" => Ok(Asterisk),
        "At" | "@" => Ok(At),
        "Ax" => Ok(Ax),
        "Backslash" | "\\" => Ok(Backslash),
        "Calculator" => Ok(Calculator),
        "Capital" => Ok(Capital),
        "Colon" | ":" => Ok(Colon),
        "Comma" | "," => Ok(Comma),
        "Convert" => Ok(Convert),
        "Equals" | "=" => Ok(Equals),
        "Grave" | "`" => Ok(Grave),
        "Kana" => Ok(Kana),
        "Kanji" => Ok(Kanji),
        "LAlt" => Ok(LAlt),
        "LBracket" | "(" => Ok(LBracket),
        "LControl" => Ok(LControl),
        "LShift" => Ok(LShift),
        "LWin" => Ok(LWin),
        "Mail" => Ok(Mail),
        "MediaSelect" => Ok(MediaSelect),
        "MediaStop" => Ok(MediaStop),
        "Minus" | "-" => Ok(Minus),
        "Mute" => Ok(Mute),
        "MyComputer" => Ok(MyComputer),
        "NavigateForward" => Ok(NavigateForward),
        "NavigateBackward" => Ok(NavigateBackward),
        "NextTrack" => Ok(NextTrack),
        "NoConvert" => Ok(NoConvert),
        "OEM102" => Ok(OEM102),
        "Period" | "." => Ok(Period),
        "PlayPause" => Ok(PlayPause),
        "Plus" | "+" => Ok(Plus),
        "Power" => Ok(Power),
        "PrevTrack" => Ok(PrevTrack),
        "RAlt" => Ok(RAlt),
        "RBracket" | ")" => Ok(RBracket),
        "RControl" => Ok(RControl),
        "RShift" => Ok(RShift),
        "RWin" => Ok(RWin),
        "Semicolon" | ";" => Ok(Semicolon),
        "Slash" | "/" => Ok(Slash),
        "Sleep" => Ok(Sleep),
        "Stop" => Ok(Stop),
        "Sysrq" => Ok(Sysrq),
        "Tab" => Ok(Tab),
        "Underline" | "_" => Ok(Underline),
        "Unlabeled" => Ok(Unlabeled),
        "VolumeDown" => Ok(VolumeDown),
        "VolumeUp" => Ok(VolumeUp),
        "Wake" => Ok(Wake),
        "WebBack" => Ok(WebBack),
        "WebFavorites" => Ok(WebFavorites),
        "WebForward" => Ok(WebForward),
        "WebHome" => Ok(WebHome),
        "WebRefresh" => Ok(WebRefresh),
        "WebSearch" => Ok(WebSearch),
        "WebStop" => Ok(WebStop),
        "Yen" => Ok(Yen),
        "Copy" => Ok(Copy),
        "Paste" => Ok(Paste),
        "Cut" => Ok(Cut),

        _ => Err(()),
    }
    .ok()
}
