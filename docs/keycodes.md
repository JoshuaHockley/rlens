# Keycodes

This file documents the form of keycodes that are expected by the `bind` API functions (see [keybinds](api.md#keybinds))

`[S-][C-][A-][L-]<keycode>`
* `S`: Shift
* `C`: Ctrl
* `A`: Alt
* `L`: Logo / Command / Super

The keycode refers to the true character being inputted, taking the shift key into account.
For example, 'shift-8' sends the `*` character, not `8`.
An exception of this is the letters keys A-Z. They are only accepted in lowercase form (`a`).

If the keypress involves the shift key, the `S` marker must be present, even if the keycode
implies the shifting itself.
For example, an input of `*` can only be matched as `S-*`, and not `*` alone.

### Keycode list

```
1
2
3
4
5
6
7
8
9
0

a
b
c
d
e
f
g
h
i
j
k
l
m
n
o
p
q
r
s
t
u
v
w
x
y
z

Esc

F1
F2
F3
F4
F5
F6
F7
F8
F9
F10
F11
F12
F13
F14
F15
F16
F17
F18
F19
F20
F21
F22
F23
F24

Snapshot
Scroll
Pause
Insert
Home
Delete
End
PageDown
PageUp

Left
Right
Up
Down

Back
Return
Space
Compose

Caret | ^

Numlock

Numpad0
Numpad1
Numpad2
Numpad3
Numpad4
Numpad5
Numpad6
Numpad7
Numpad8
Numpad9

NumpadAdd
NumpadDivide
NumpadDecimal
NumpadComma
NumpadEnter
NumpadEquals
NumpadMultiply
NumpadSubtract

AbntC1
AbntC2
Apostrophe | '
Apps
Asterisk | *
At | @
Ax
Backslash | \
Calculator
Capital
Colon | :
Comma | ,
Convert
Equals | =
Grave | `
Kana
Kanji
LAlt
LBracket | (
LControl
LShift
LWin
Mail
MediaSelect
MediaStop
Minus | -
Mute
MyComputer
NavigateForward
NavigateBackward
NextTrack
NoConvert
OEM102
Period | .
PlayPause
Plus | +
Power
PrevTrack
RAlt
RBracket | )
RControl
RShift
RWin
Semicolon | ;
Slash | /
Sleep
Stop
Sysrq
Tab
Underline | _
Unlabeled
VolumeDown
VolumeUp
Wake
WebBack
WebFavorites
WebForward
WebHome
WebRefresh
WebSearch
WebStop
Yen
Copy
Paste
Cut
```
