use anyhow::Result;
use keycode::KeyMappingCode;
use keycode::KeyMappingCode::*;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombination {
    pub key: Key,
    pub modifiers: KeyCombinationModifiers,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombinationModifiers {
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

impl KeyCombination {
    pub fn serialize(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Sorted alphabetically for storage stability
        if self.modifiers.alt {
            parts.push(AltLeft.to_string());
        }
        if self.modifiers.ctrl {
            parts.push(ControlLeft.to_string());
        }
        if self.modifiers.meta {
            parts.push(MetaLeft.to_string());
        }
        if self.modifiers.shift {
            parts.push(ShiftLeft.to_string());
        }

        let key = self.key.0.to_string();
        parts.push(key);

        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub KeyMappingCode);

impl Key {
    pub fn modifiers() -> Vec<Self> {
        vec![
            ShiftLeft,
            ShiftRight,
            ControlLeft,
            ControlRight,
            MetaLeft,
            MetaRight,
            AltLeft,
            AltRight,
            Fn,
        ]
        .into_iter()
        .map(Key)
        .collect()
    }

    pub fn non_shift_modifiers() -> Vec<Self> {
        vec![
            ControlLeft,
            ControlRight,
            MetaLeft,
            MetaRight,
            AltLeft,
            AltRight,
            Fn,
        ]
        .into_iter()
        .map(Key)
        .collect()
    }

    // We do manual mapping for best performance
    pub fn is_digit(&self) -> bool {
        matches!(
            self.0,
            Digit1 | Digit2 | Digit3 | Digit4 | Digit5 | Digit6 | Digit7 | Digit8 | Digit9 | Digit0
        )
    }

    pub fn is_letter(&self) -> bool {
        matches!(
            self.0,
            KeyA | KeyB
                | KeyC
                | KeyD
                | KeyE
                | KeyF
                | KeyG
                | KeyH
                | KeyI
                | KeyJ
                | KeyK
                | KeyL
                | KeyM
                | KeyN
                | KeyO
                | KeyP
                | KeyQ
                | KeyR
                | KeyS
                | KeyT
                | KeyU
                | KeyV
                | KeyW
                | KeyX
                | KeyY
                | KeyZ
        )
    }

    pub fn to_char(&self, shift: bool) -> Option<char> {
        let char = match self.0 {
            KeyA => {
                if shift {
                    'A'
                } else {
                    'a'
                }
            }
            KeyB => {
                if shift {
                    'B'
                } else {
                    'b'
                }
            }
            KeyC => {
                if shift {
                    'C'
                } else {
                    'c'
                }
            }
            KeyD => {
                if shift {
                    'D'
                } else {
                    'd'
                }
            }
            KeyE => {
                if shift {
                    'E'
                } else {
                    'e'
                }
            }
            KeyF => {
                if shift {
                    'F'
                } else {
                    'f'
                }
            }
            KeyG => {
                if shift {
                    'G'
                } else {
                    'g'
                }
            }
            KeyH => {
                if shift {
                    'H'
                } else {
                    'h'
                }
            }
            KeyI => {
                if shift {
                    'I'
                } else {
                    'i'
                }
            }
            KeyJ => {
                if shift {
                    'J'
                } else {
                    'j'
                }
            }
            KeyK => {
                if shift {
                    'K'
                } else {
                    'k'
                }
            }
            KeyL => {
                if shift {
                    'L'
                } else {
                    'l'
                }
            }
            KeyM => {
                if shift {
                    'M'
                } else {
                    'm'
                }
            }
            KeyN => {
                if shift {
                    'N'
                } else {
                    'n'
                }
            }
            KeyO => {
                if shift {
                    'O'
                } else {
                    'o'
                }
            }
            KeyP => {
                if shift {
                    'P'
                } else {
                    'p'
                }
            }
            KeyQ => {
                if shift {
                    'Q'
                } else {
                    'q'
                }
            }
            KeyR => {
                if shift {
                    'R'
                } else {
                    'r'
                }
            }
            KeyS => {
                if shift {
                    'S'
                } else {
                    's'
                }
            }
            KeyT => {
                if shift {
                    'T'
                } else {
                    't'
                }
            }
            KeyU => {
                if shift {
                    'U'
                } else {
                    'u'
                }
            }
            KeyV => {
                if shift {
                    'V'
                } else {
                    'v'
                }
            }
            KeyW => {
                if shift {
                    'W'
                } else {
                    'w'
                }
            }
            KeyX => {
                if shift {
                    'X'
                } else {
                    'x'
                }
            }
            KeyY => {
                if shift {
                    'Y'
                } else {
                    'y'
                }
            }
            KeyZ => {
                if shift {
                    'Z'
                } else {
                    'z'
                }
            }
            Backquote => {
                if shift {
                    '~'
                } else {
                    '`'
                }
            }
            Digit1 => {
                if shift {
                    '!'
                } else {
                    '1'
                }
            }
            Digit2 => {
                if shift {
                    '@'
                } else {
                    '2'
                }
            }
            Digit3 => {
                if shift {
                    '#'
                } else {
                    '3'
                }
            }
            Digit4 => {
                if shift {
                    '$'
                } else {
                    '4'
                }
            }
            Digit5 => {
                if shift {
                    '%'
                } else {
                    '5'
                }
            }
            Digit6 => {
                if shift {
                    '^'
                } else {
                    '6'
                }
            }
            Digit7 => {
                if shift {
                    '&'
                } else {
                    '7'
                }
            }
            Digit8 => {
                if shift {
                    '*'
                } else {
                    '8'
                }
            }
            Digit9 => {
                if shift {
                    '('
                } else {
                    '9'
                }
            }
            Digit0 => {
                if shift {
                    ')'
                } else {
                    '0'
                }
            }
            Minus => {
                if shift {
                    '-'
                } else {
                    '_'
                }
            }
            Equal => {
                if shift {
                    '+'
                } else {
                    '='
                }
            }
            BracketLeft => {
                if shift {
                    '{'
                } else {
                    '['
                }
            }
            BracketRight => {
                if shift {
                    '}'
                } else {
                    ']'
                }
            }
            Backslash => {
                if shift {
                    '|'
                } else {
                    '\\'
                }
            }
            Semicolon => {
                if shift {
                    ':'
                } else {
                    ';'
                }
            }
            Quote => {
                if shift {
                    '"'
                } else {
                    '\''
                }
            }
            Comma => {
                if shift {
                    '<'
                } else {
                    ','
                }
            }
            Period => {
                if shift {
                    '>'
                } else {
                    '.'
                }
            }
            Slash => {
                if shift {
                    '?'
                } else {
                    '/'
                }
            }
            _ => return None,
        };

        Some(char)
    }

    pub fn parse_sequence(sequence: &str) -> Result<Vec<Key>> {
        let mut keys = Vec::new();
        for key in sequence.chars().map(|c| Key::from_str(&c.to_string())) {
            keys.push(
                key.map_err(|_| anyhow::anyhow!("Failed to parse chord sequence: {}", sequence))?,
            );
        }

        Ok(keys)
    }
}

impl FromStr for Key {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(first_char) = value.chars().next() else {
            return Err(anyhow::anyhow!("Invalid key: empty string"));
        };

        if first_char.is_ascii_uppercase() {
            KeyMappingCode::from_str(value)
                .map(Key)
                .map_err(|_| anyhow::anyhow!("Invalid key mapping id: {}", value))
        } else {
            // Lowercase means we use an abbreviated form
            // https://github.com/electron/electron/blob/26a3a8679a063623cf7e6bc1f5e07042fa953d7a/shell/common/keyboard_util.cc#L29
            let code = match value {
                "alt" | "opt" | "option" => AltLeft,
                "altgr" => AltRight,
                "backspace" => Backspace,
                "capslock" => CapsLock,
                "cmd" | "command" | "meta" | "super" => MetaLeft,
                "cmdorctrl" | "commandorcontrol" => MetaLeft,
                "control" | "ctrl" => ControlLeft,
                "delete" => Delete,
                "down" => ArrowDown,
                "end" => End,
                "enter" | "return" => Enter,
                "esc" | "escape" => Escape,
                "fn" | "function" => Fn,
                "f1" => F1,
                "f2" => F2,
                "f3" => F3,
                "f4" => F4,
                "f5" => F5,
                "f6" => F6,
                "f7" => F7,
                "f8" => F8,
                "f9" => F9,
                "f10" => F10,
                "f11" => F11,
                "f12" => F12,
                "f13" => F13,
                "f14" => F14,
                "f15" => F15,
                "f16" => F16,
                "f17" => F17,
                "f18" => F18,
                "f19" => F19,
                "f20" => F20,
                "f21" => F21,
                "f22" => F22,
                "f23" => F23,
                "f24" => F24,
                "home" => Home,
                "insert" => Insert,
                "left" => ArrowLeft,
                "medianexttrack" => MediaTrackNext,
                "mediaplaypause" => MediaPlayPause,
                "mediaprevioustrack" => MediaTrackPrevious,
                "mediastop" => MediaStop,
                "num0" => Numpad0,
                "num1" => Numpad1,
                "num2" => Numpad2,
                "num3" => Numpad3,
                "num4" => Numpad4,
                "num5" => Numpad5,
                "num6" => Numpad6,
                "num7" => Numpad7,
                "num8" => Numpad8,
                "num9" => Numpad9,
                "numadd" => NumpadAdd,
                "numdec" => NumpadDecimal,
                "numdiv" => NumpadDivide,
                "numlock" => NumLock,
                "nummult" => NumpadMultiply,
                "numsub" => NumpadSubtract,
                "pagedown" | "pgdown" => PageDown,
                "pageup" | "pgup" => PageUp,
                "plus" => Equal, // plus(+) is usually shift+'=' on en-US layout
                "printscreen" => PrintScreen,
                "right" => ArrowRight,
                "scrolllock" => ScrollLock,
                "shift" => ShiftLeft,
                "space" => Space,
                "tab" => Tab,
                "up" => ArrowUp,
                "volumedown" => AudioVolumeDown,
                "volumemute" => AudioVolumeMute,
                "volumeup" => AudioVolumeUp,
                "a" | "A" => KeyA,
                "b" | "B" => KeyB,
                "c" | "C" => KeyC,
                "d" | "D" => KeyD,
                "e" | "E" => KeyE,
                "f" | "F" => KeyF,
                "g" | "G" => KeyG,
                "h" | "H" => KeyH,
                "i" | "I" => KeyI,
                "j" | "J" => KeyJ,
                "k" | "K" => KeyK,
                "l" | "L" => KeyL,
                "m" | "M" => KeyM,
                "n" | "N" => KeyN,
                "o" | "O" => KeyO,
                "p" | "P" => KeyP,
                "q" | "Q" => KeyQ,
                "r" | "R" => KeyR,
                "s" | "S" => KeyS,
                "t" | "T" => KeyT,
                "u" | "U" => KeyU,
                "v" | "V" => KeyV,
                "w" | "W" => KeyW,
                "x" | "X" => KeyX,
                "y" | "Y" => KeyY,
                "z" | "Z" => KeyZ,
                "1" => Digit1,
                "2" => Digit2,
                "3" => Digit3,
                "4" => Digit4,
                "5" => Digit5,
                "6" => Digit6,
                "7" => Digit7,
                "8" => Digit8,
                "9" => Digit9,
                "0" => Digit0,
                "-" | "minus" => Minus,
                "=" | "equal" | "equals" => Equal,
                "[" => BracketLeft,
                "]" => BracketRight,
                "\\" | "backslash" => Backslash,
                ";" | "semicolon" => Semicolon,
                "'" | "quote" => Quote,
                "," | "comma" => Comma,
                "." | "period" => Period,
                "/" | "slash" => Slash,
                "`" | "backtick" => Backquote,
                _ => return Err(anyhow::anyhow!("Unknown key: {}", value)),
            };

            Ok(Key(code))
        }
    }
}

impl TryFrom<Key> for mac_keycode::Key {
    type Error = anyhow::Error;

    fn try_from(key: Key) -> Result<Self> {
        use mac_keycode::Key as K;

        let mac_key = match key.0 {
            KeyA => K::A,
            KeyS => K::S,
            KeyD => K::D,
            KeyF => K::F,
            KeyH => K::H,
            KeyG => K::G,
            KeyZ => K::Z,
            KeyX => K::X,
            KeyC => K::C,
            KeyV => K::V,
            IntlBackslash => K::ISOSection,
            KeyB => K::B,
            KeyQ => K::Q,
            KeyW => K::W,
            KeyE => K::E,
            KeyR => K::R,
            KeyY => K::Y,
            KeyT => K::T,
            Digit1 => K::Digit1,
            Digit2 => K::Digit2,
            Digit3 => K::Digit3,
            Digit4 => K::Digit4,
            Digit6 => K::Digit6,
            Digit5 => K::Digit5,
            Equal => K::Equal,
            Digit9 => K::Digit9,
            Digit7 => K::Digit7,
            Minus => K::Minus,
            Digit8 => K::Digit8,
            Digit0 => K::Digit0,
            BracketRight => K::RightBracket,
            KeyO => K::O,
            KeyU => K::U,
            BracketLeft => K::LeftBracket,
            KeyI => K::I,
            KeyP => K::P,
            Enter => K::Return,
            KeyL => K::L,
            KeyJ => K::J,
            Quote => K::Quote,
            KeyK => K::K,
            Semicolon => K::Semicolon,
            Backslash => K::Backslash,
            Comma => K::Comma,
            Slash => K::Slash,
            KeyN => K::N,
            KeyM => K::M,
            Period => K::Period,
            Tab => K::Tab,
            Space => K::Space,
            Backquote => K::Grave,
            Backspace => K::Delete,
            Escape => K::Escape,
            MetaRight => K::RightCommand,
            MetaLeft => K::Command,
            ShiftLeft => K::Shift,
            CapsLock => K::CapsLock,
            AltLeft => K::Option,
            ControlLeft => K::Control,
            ShiftRight => K::RightShift,
            AltRight => K::RightOption,
            ControlRight => K::RightControl,
            Fn => K::Function,
            F17 => K::F17,
            NumpadDecimal => K::KeypadDecimal,
            NumpadMultiply => K::KeypadMultiply,
            NumpadAdd => K::KeypadPlus,
            AudioVolumeUp => K::VolumeUp,
            AudioVolumeDown => K::VolumeDown,
            AudioVolumeMute => K::Mute,
            NumpadDivide => K::KeypadDivide,
            NumpadEnter => K::KeypadEnter,
            NumpadSubtract => K::KeypadMinus,
            F18 => K::F18,
            F19 => K::F19,
            NumpadEqual => K::KeypadEquals,
            Numpad0 => K::Keypad0,
            Numpad1 => K::Keypad1,
            Numpad2 => K::Keypad2,
            Numpad3 => K::Keypad3,
            Numpad4 => K::Keypad4,
            Numpad5 => K::Keypad5,
            Numpad6 => K::Keypad6,
            Numpad7 => K::Keypad7,
            F20 => K::F20,
            Numpad8 => K::Keypad8,
            Numpad9 => K::Keypad9,
            F5 => K::F5,
            F6 => K::F6,
            F7 => K::F7,
            F3 => K::F3,
            F8 => K::F8,
            F9 => K::F9,
            F11 => K::F11,
            F13 => K::F13,
            F16 => K::F16,
            F14 => K::F14,
            F10 => K::F10,
            F12 => K::F12,
            F15 => K::F15,
            Insert => K::Help,
            Home => K::Home,
            PageUp => K::PageUp,
            Delete => K::ForwardDelete,
            F4 => K::F4,
            End => K::End,
            F2 => K::F2,
            PageDown => K::PageDown,
            F1 => K::F1,
            ArrowLeft => K::LeftArrow,
            ArrowRight => K::RightArrow,
            ArrowDown => K::DownArrow,
            ArrowUp => K::UpArrow,
            _ => return Err(anyhow::anyhow!("Unknown key: {:?}", key)),
        };

        Ok(mac_key)
    }
}

impl TryFrom<mac_keycode::Key> for Key {
    type Error = anyhow::Error;

    fn try_from(key: mac_keycode::Key) -> Result<Self> {
        use mac_keycode::Key as K;

        Ok(Key(match key {
            K::A => KeyA,
            K::S => KeyS,
            K::D => KeyD,
            K::F => KeyF,
            K::H => KeyH,
            K::G => KeyG,
            K::Z => KeyZ,
            K::X => KeyX,
            K::C => KeyC,
            K::V => KeyV,
            K::ISOSection => IntlBackslash,
            K::B => KeyB,
            K::Q => KeyQ,
            K::W => KeyW,
            K::E => KeyE,
            K::R => KeyR,
            K::Y => KeyY,
            K::T => KeyT,

            K::Digit1 => Digit1,
            K::Digit2 => Digit2,
            K::Digit3 => Digit3,
            K::Digit4 => Digit4,
            K::Digit5 => Digit5,
            K::Digit6 => Digit6,
            K::Digit7 => Digit7,
            K::Digit8 => Digit8,
            K::Digit9 => Digit9,
            K::Digit0 => Digit0,

            K::Equal => Equal,
            K::Minus => Minus,

            K::LeftBracket => BracketLeft,
            K::RightBracket => BracketRight,

            K::O => KeyO,
            K::U => KeyU,
            K::I => KeyI,
            K::P => KeyP,

            K::Return => Enter,
            K::L => KeyL,
            K::J => KeyJ,
            K::Quote => Quote,
            K::K => KeyK,
            K::Semicolon => Semicolon,
            K::Backslash => Backslash,

            K::Comma => Comma,
            K::Slash => Slash,
            K::N => KeyN,
            K::M => KeyM,
            K::Period => Period,

            K::Tab => Tab,
            K::Space => Space,
            K::Grave => Backquote,
            K::Delete => Backspace,
            K::Escape => Escape,

            K::Command => MetaLeft,
            K::RightCommand => MetaRight,
            K::Shift => ShiftLeft,
            K::RightShift => ShiftRight,
            K::Option => AltLeft,
            K::RightOption => AltRight,
            K::Control => ControlLeft,
            K::RightControl => ControlRight,

            K::Function => Fn,

            K::F1 => F1,
            K::F2 => F2,
            K::F3 => F3,
            K::F4 => F4,
            K::F5 => F5,
            K::F6 => F6,
            K::F7 => F7,
            K::F8 => F8,
            K::F9 => F9,
            K::F10 => F10,
            K::F11 => F11,
            K::F12 => F12,
            K::F13 => F13,
            K::F14 => F14,
            K::F15 => F15,
            K::F16 => F16,
            K::F17 => F17,
            K::F18 => F18,
            K::F19 => F19,
            K::F20 => F20,

            K::Keypad0 => Numpad0,
            K::Keypad1 => Numpad1,
            K::Keypad2 => Numpad2,
            K::Keypad3 => Numpad3,
            K::Keypad4 => Numpad4,
            K::Keypad5 => Numpad5,
            K::Keypad6 => Numpad6,
            K::Keypad7 => Numpad7,
            K::Keypad8 => Numpad8,
            K::Keypad9 => Numpad9,

            K::KeypadDecimal => NumpadDecimal,
            K::KeypadMultiply => NumpadMultiply,
            K::KeypadPlus => NumpadAdd,
            K::KeypadDivide => NumpadDivide,
            K::KeypadEnter => NumpadEnter,
            K::KeypadMinus => NumpadSubtract,
            K::KeypadEquals => NumpadEqual,

            K::VolumeUp => AudioVolumeUp,
            K::VolumeDown => AudioVolumeDown,
            K::Mute => AudioVolumeMute,

            K::Help => Insert,
            K::Home => Home,
            K::PageUp => PageUp,
            K::ForwardDelete => Delete,
            K::End => End,
            K::PageDown => PageDown,

            K::LeftArrow => ArrowLeft,
            K::RightArrow => ArrowRight,
            K::DownArrow => ArrowDown,
            K::UpArrow => ArrowUp,
            K::CapsLock => CapsLock,
            _ => return Err(anyhow::anyhow!("Unknown key: {:?}", key)),
        }))
    }
}

impl TryFrom<Key> for rdev::Key {
    type Error = anyhow::Error;

    fn try_from(key: Key) -> Result<Self, Self::Error> {
        use rdev::Key as K;
        let rdev_key = match key.0 {
            Backspace => K::Backspace,
            CapsLock => K::CapsLock,
            ControlLeft => K::ControlLeft,
            ControlRight => K::ControlRight,
            Delete => K::Delete,
            ArrowDown => K::DownArrow,
            End => K::End,
            Escape => K::Escape,
            F1 => K::F1,
            F2 => K::F2,
            F3 => K::F3,
            F4 => K::F4,
            F5 => K::F5,
            F6 => K::F6,
            F7 => K::F7,
            F8 => K::F8,
            F9 => K::F9,
            F10 => K::F10,
            F11 => K::F11,
            F12 => K::F12,
            // For some reason, simulate(K::F17) doesn't work on macOS
            F13 => K::Unknown(0x69),
            F14 => K::Unknown(0x6B),
            F15 => K::Unknown(0x71),
            F16 => K::Unknown(0x6A),
            F17 => K::Unknown(0x40),
            F18 => K::Unknown(0x4F),
            F19 => K::Unknown(0x50),
            F20 => K::Unknown(0x5A),
            F21 => K::F21,
            F22 => K::F22,
            F23 => K::F23,
            F24 => K::F24,
            Home => K::Home,
            ArrowLeft => K::LeftArrow,
            MetaLeft => K::MetaLeft,
            MetaRight => K::MetaRight,
            PageDown => K::PageDown,
            PageUp => K::PageUp,
            Enter => K::Return,
            ArrowRight => K::RightArrow,
            ShiftLeft => K::ShiftLeft,
            ShiftRight => K::ShiftRight,
            AltLeft => K::Alt,
            AltRight => K::AltGr,
            Space => K::Space,
            Tab => K::Tab,
            ArrowUp => K::UpArrow,
            PrintScreen => K::PrintScreen,
            ScrollLock => K::ScrollLock,
            Pause => K::Pause,
            NumLock => K::NumLock,
            Backquote => K::BackQuote,
            Digit1 => K::Num1,
            Digit2 => K::Num2,
            Digit3 => K::Num3,
            Digit4 => K::Num4,
            Digit5 => K::Num5,
            Digit6 => K::Num6,
            Digit7 => K::Num7,
            Digit8 => K::Num8,
            Digit9 => K::Num9,
            Digit0 => K::Num0,
            Minus => K::Minus,
            Equal => K::Equal,
            KeyQ => K::KeyQ,
            KeyW => K::KeyW,
            KeyE => K::KeyE,
            KeyR => K::KeyR,
            KeyT => K::KeyT,
            KeyY => K::KeyY,
            KeyU => K::KeyU,
            KeyI => K::KeyI,
            KeyO => K::KeyO,
            KeyP => K::KeyP,
            BracketLeft => K::LeftBracket,
            BracketRight => K::RightBracket,
            KeyA => K::KeyA,
            KeyS => K::KeyS,
            KeyD => K::KeyD,
            KeyF => K::KeyF,
            KeyG => K::KeyG,
            KeyH => K::KeyH,
            KeyJ => K::KeyJ,
            KeyK => K::KeyK,
            KeyL => K::KeyL,
            Semicolon => K::SemiColon,
            Quote => K::Quote,
            Backslash => K::BackSlash,
            IntlBackslash => K::IntlBackslash,
            KeyZ => K::KeyZ,
            KeyX => K::KeyX,
            KeyC => K::KeyC,
            KeyV => K::KeyV,
            KeyB => K::KeyB,
            KeyN => K::KeyN,
            KeyM => K::KeyM,
            Comma => K::Comma,
            Period => K::Dot,
            Slash => K::Slash,
            Insert => K::Insert,
            NumpadEnter => K::KpReturn,
            NumpadSubtract => K::KpMinus,
            NumpadAdd => K::KpPlus,
            NumpadMultiply => K::KpMultiply,
            NumpadDivide => K::KpDivide,
            Numpad0 => K::Kp0,
            Numpad1 => K::Kp1,
            Numpad2 => K::Kp2,
            Numpad3 => K::Kp3,
            Numpad4 => K::Kp4,
            Numpad5 => K::Kp5,
            Numpad6 => K::Kp6,
            Numpad7 => K::Kp7,
            Numpad8 => K::Kp8,
            Numpad9 => K::Kp9,
            NumpadBackspace => K::KpDelete,
            Fn => K::Function,
            AudioVolumeUp => K::VolumeUp,
            AudioVolumeDown => K::VolumeDown,
            AudioVolumeMute => K::VolumeMute,
            BrightnessUp => K::BrightnessUp,
            BrightnessDown => K::BrightnessDown,
            MediaTrackPrevious => K::PreviousTrack,
            MediaPlayPause => K::PlayPause,
            MediaPlay => K::PlayCd,
            MediaTrackNext => K::NextTrack,
            _ => return Err(anyhow::anyhow!("Unknown key: {:?}", key)),
        };

        Ok(rdev_key)
    }
}

impl TryFrom<rdev::Key> for Key {
    type Error = anyhow::Error;

    fn try_from(key: rdev::Key) -> Result<Self, Self::Error> {
        use rdev::Key as K;
        let code = match key {
            K::Alt => AltLeft,
            K::AltGr => AltRight,
            K::Backspace => Backspace,
            K::CapsLock => CapsLock,
            K::ControlLeft => ControlLeft,
            K::ControlRight => ControlRight,
            K::Delete => Delete,
            K::DownArrow => ArrowDown,
            K::End => End,
            K::Escape => Escape,
            K::F1 => F1,
            K::F2 => F2,
            K::F3 => F3,
            K::F4 => F4,
            K::F5 => F5,
            K::F6 => F6,
            K::F7 => F7,
            K::F8 => F8,
            K::F9 => F9,
            K::F10 => F10,
            K::F11 => F11,
            K::F12 => F12,
            K::F13 => F13,
            K::F14 => F14,
            K::F15 => F15,
            K::F16 => F16,
            K::F17 => F17,
            K::F18 => F18,
            K::F19 => F19,
            K::F20 => F20,
            K::F21 => F21,
            K::F22 => F22,
            K::F23 => F23,
            K::F24 => F24,
            K::Home => Home,
            K::LeftArrow => ArrowLeft,
            K::MetaLeft => MetaLeft,
            K::MetaRight => MetaRight,
            K::PageDown => PageDown,
            K::PageUp => PageUp,
            K::Return => Enter,
            K::RightArrow => ArrowRight,
            K::ShiftLeft => ShiftLeft,
            K::ShiftRight => ShiftRight,
            K::Space => Space,
            K::Tab => Tab,
            K::UpArrow => ArrowUp,
            K::PrintScreen => PrintScreen,
            K::ScrollLock => ScrollLock,
            K::Pause => Pause,
            K::NumLock => NumLock,
            K::BackQuote => Backquote,
            K::Num1 => Digit1,
            K::Num2 => Digit2,
            K::Num3 => Digit3,
            K::Num4 => Digit4,
            K::Num5 => Digit5,
            K::Num6 => Digit6,
            K::Num7 => Digit7,
            K::Num8 => Digit8,
            K::Num9 => Digit9,
            K::Num0 => Digit0,
            K::Minus => Minus,
            K::Equal => Equal,
            K::KeyQ => KeyQ,
            K::KeyW => KeyW,
            K::KeyE => KeyE,
            K::KeyR => KeyR,
            K::KeyT => KeyT,
            K::KeyY => KeyY,
            K::KeyU => KeyU,
            K::KeyI => KeyI,
            K::KeyO => KeyO,
            K::KeyP => KeyP,
            K::LeftBracket => BracketLeft,
            K::RightBracket => BracketRight,
            K::KeyA => KeyA,
            K::KeyS => KeyS,
            K::KeyD => KeyD,
            K::KeyF => KeyF,
            K::KeyG => KeyG,
            K::KeyH => KeyH,
            K::KeyJ => KeyJ,
            K::KeyK => KeyK,
            K::KeyL => KeyL,
            K::SemiColon => Semicolon,
            K::Quote => Quote,
            K::BackSlash => Backslash,
            K::IntlBackslash => IntlBackslash,
            K::KeyZ => KeyZ,
            K::KeyX => KeyX,
            K::KeyC => KeyC,
            K::KeyV => KeyV,
            K::KeyB => KeyB,
            K::KeyN => KeyN,
            K::KeyM => KeyM,
            K::Comma => Comma,
            K::Dot => Period,
            K::Slash => Slash,
            K::Insert => Insert,
            K::KpReturn => NumpadEnter,
            K::KpMinus => NumpadSubtract,
            K::KpPlus => NumpadAdd,
            K::KpMultiply => NumpadMultiply,
            K::KpDivide => NumpadDivide,
            K::Kp0 => Numpad0,
            K::Kp1 => Numpad1,
            K::Kp2 => Numpad2,
            K::Kp3 => Numpad3,
            K::Kp4 => Numpad4,
            K::Kp5 => Numpad5,
            K::Kp6 => Numpad6,
            K::Kp7 => Numpad7,
            K::Kp8 => Numpad8,
            K::Kp9 => Numpad9,
            K::KpDelete => NumpadBackspace,
            K::Function => Fn,
            K::VolumeUp => AudioVolumeUp,
            K::VolumeDown => AudioVolumeDown,
            K::VolumeMute => AudioVolumeMute,
            K::BrightnessUp => BrightnessUp,
            K::BrightnessDown => BrightnessDown,
            K::PreviousTrack => MediaTrackPrevious,
            K::PlayPause => MediaPlayPause,
            K::PlayCd => MediaPlay,
            K::NextTrack => MediaTrackNext,
            _ => return Err(anyhow::anyhow!("Unknown key: {:?}", key)),
        };

        Ok(Key(code))
    }
}

impl From<device_query::Keycode> for Key {
    fn from(key: device_query::Keycode) -> Self {
        use device_query::Keycode as K;

        match key {
            K::Key0 => Key(Digit0),
            K::Key1 => Key(Digit1),
            K::Key2 => Key(Digit2),
            K::Key3 => Key(Digit3),
            K::Key4 => Key(Digit4),
            K::Key5 => Key(Digit5),
            K::Key6 => Key(Digit6),
            K::Key7 => Key(Digit7),
            K::Key8 => Key(Digit8),
            K::Key9 => Key(Digit9),
            K::A => Key(KeyA),
            K::B => Key(KeyB),
            K::C => Key(KeyC),
            K::D => Key(KeyD),
            K::E => Key(KeyE),
            K::F => Key(KeyF),
            K::G => Key(KeyG),
            K::H => Key(KeyH),
            K::I => Key(KeyI),
            K::J => Key(KeyJ),
            K::K => Key(KeyK),
            K::L => Key(KeyL),
            K::M => Key(KeyM),
            K::N => Key(KeyN),
            K::O => Key(KeyO),
            K::P => Key(KeyP),
            K::Q => Key(KeyQ),
            K::R => Key(KeyR),
            K::S => Key(KeyS),
            K::T => Key(KeyT),
            K::U => Key(KeyU),
            K::V => Key(KeyV),
            K::W => Key(KeyW),
            K::X => Key(KeyX),
            K::Y => Key(KeyY),
            K::Z => Key(KeyZ),
            K::F1 => Key(F1),
            K::F2 => Key(F2),
            K::F3 => Key(F3),
            K::F4 => Key(F4),
            K::F5 => Key(F5),
            K::F6 => Key(F6),
            K::F7 => Key(F7),
            K::F8 => Key(F8),
            K::F9 => Key(F9),
            K::F10 => Key(F10),
            K::F11 => Key(F11),
            K::F12 => Key(F12),
            K::F13 => Key(F13),
            K::F14 => Key(F14),
            K::F15 => Key(F15),
            K::F16 => Key(F16),
            K::F17 => Key(F17),
            K::F18 => Key(F18),
            K::F19 => Key(F19),
            K::F20 => Key(F20),
            K::Escape => Key(Escape),
            K::Space => Key(Space),
            K::LControl => Key(ControlLeft),
            K::RControl => Key(ControlRight),
            K::LShift => Key(ShiftLeft),
            K::RShift => Key(ShiftRight),
            K::LAlt | K::LOption => Key(AltLeft),
            K::RAlt | K::ROption => Key(AltRight),
            K::Command | K::LMeta => Key(MetaLeft),
            K::RCommand | K::RMeta => Key(MetaRight),
            K::Enter => Key(Enter),
            K::Up => Key(ArrowUp),
            K::Down => Key(ArrowDown),
            K::Left => Key(ArrowLeft),
            K::Right => Key(ArrowRight),
            K::Backspace => Key(Backspace),
            K::CapsLock => Key(CapsLock),
            K::Tab => Key(Tab),
            K::Home => Key(Home),
            K::End => Key(End),
            K::PageUp => Key(PageUp),
            K::PageDown => Key(PageDown),
            K::Insert => Key(Insert),
            K::Delete => Key(Delete),
            K::Numpad0 => Key(Numpad0),
            K::Numpad1 => Key(Numpad1),
            K::Numpad2 => Key(Numpad2),
            K::Numpad3 => Key(Numpad3),
            K::Numpad4 => Key(Numpad4),
            K::Numpad5 => Key(Numpad5),
            K::Numpad6 => Key(Numpad6),
            K::Numpad7 => Key(Numpad7),
            K::Numpad8 => Key(Numpad8),
            K::Numpad9 => Key(Numpad9),
            K::NumpadSubtract => Key(NumpadSubtract),
            K::NumpadAdd => Key(NumpadAdd),
            K::NumpadDivide => Key(NumpadDivide),
            K::NumpadMultiply => Key(NumpadMultiply),
            K::NumpadEquals => Key(NumpadEqual),
            K::NumpadEnter => Key(NumpadEnter),
            K::NumpadDecimal => Key(NumpadDecimal),
            K::Grave => Key(Backquote),
            K::Minus => Key(Minus),
            K::Equal => Key(Equal),
            K::LeftBracket => Key(BracketLeft),
            K::RightBracket => Key(BracketRight),
            K::BackSlash => Key(Backslash),
            K::Semicolon => Key(Semicolon),
            K::Apostrophe => Key(Quote),
            K::Comma => Key(Comma),
            K::Dot => Key(Period),
            K::Slash => Key(Slash),
        }
    }
}

// impl TryFrom<Key> for enigo::Key {
//     type Error = anyhow::Error;
//
//     fn try_from(key: Key) -> Result<Self, Self::Error> {
//         use enigo::Key as K;
//         let enigo_key = match key.0 {
//             AltLeft => K::Option,
//             AltRight => K::ROption,
//             Backspace => K::Backspace,
//             CapsLock => K::CapsLock,
//             ControlLeft => K::LControl,
//             ControlRight => K::RControl,
//             Delete => K::Delete,
//             ArrowDown => K::DownArrow,
//             End => K::End,
//             Escape => K::Escape,
//             F1 => K::F1,
//             F10 => K::F10,
//             F11 => K::F11,
//             F12 => K::F12,
//             F13 => K::F13,
//             F14 => K::F14,
//             F15 => K::F15,
//             F16 => K::F16,
//             F17 => K::F17,
//             F18 => K::F18,
//             F19 => K::F19,
//             F20 => K::F20,
//             F2 => K::F2,
//             F3 => K::F3,
//             F4 => K::F4,
//             F5 => K::F5,
//             F6 => K::F6,
//             F7 => K::F7,
//             F8 => K::F8,
//             F9 => K::F9,
//             Home => K::Home,
//             ArrowLeft => K::LeftArrow,
//             MetaLeft => K::Meta,
//             MetaRight => K::RCommand,
//             PageDown => K::PageDown,
//             PageUp => K::PageUp,
//             Enter => K::Return,
//             ArrowRight => K::RightArrow,
//             ShiftLeft => K::LShift,
//             ShiftRight => K::RShift,
//             Space => K::Space,
//             Tab => K::Tab,
//             ArrowUp => K::UpArrow,
//             Digit1 => K::Unicode('1'),
//             Digit2 => K::Unicode('2'),
//             Digit3 => K::Unicode('3'),
//             Digit4 => K::Unicode('4'),
//             Digit5 => K::Unicode('5'),
//             Digit6 => K::Unicode('6'),
//             Digit7 => K::Unicode('7'),
//             Digit8 => K::Unicode('8'),
//             Digit9 => K::Unicode('9'),
//             Digit0 => K::Unicode('0'),
//             Minus => K::Subtract,
//             Equal => K::Unicode('='),
//             KeyQ => K::Unicode('q'),
//             KeyW => K::Unicode('w'),
//             KeyE => K::Unicode('e'),
//             KeyR => K::Unicode('r'),
//             KeyT => K::Unicode('t'),
//             KeyY => K::Unicode('y'),
//             KeyU => K::Unicode('u'),
//             KeyI => K::Unicode('i'),
//             KeyO => K::Unicode('o'),
//             KeyP => K::Unicode('p'),
//             BracketLeft => K::Unicode('['),
//             BracketRight => K::Unicode(']'),
//             KeyA => K::Unicode('a'),
//             KeyS => K::Unicode('s'),
//             KeyD => K::Unicode('d'),
//             KeyF => K::Unicode('f'),
//             KeyG => K::Unicode('g'),
//             KeyH => K::Unicode('h'),
//             KeyJ => K::Unicode('j'),
//             KeyK => K::Unicode('k'),
//             KeyL => K::Unicode('l'),
//             Semicolon => K::Unicode(';'),
//             Quote => K::Unicode('"'),
//             Backslash => K::Unicode('\\'),
//             KeyZ => K::Unicode('z'),
//             KeyX => K::Unicode('x'),
//             KeyC => K::Unicode('c'),
//             KeyV => K::Unicode('v'),
//             KeyB => K::Unicode('b'),
//             KeyN => K::Unicode('n'),
//             KeyM => K::Unicode('m'),
//             Comma => K::Unicode(','),
//             Period => K::Unicode('.'),
//             Slash => K::Unicode('/'),
//             Numpad0 => K::Numpad0,
//             Numpad1 => K::Numpad1,
//             Numpad2 => K::Numpad2,
//             Numpad3 => K::Numpad3,
//             Numpad4 => K::Numpad4,
//             Numpad5 => K::Numpad5,
//             Numpad6 => K::Numpad6,
//             Numpad7 => K::Numpad7,
//             Numpad8 => K::Numpad8,
//             Numpad9 => K::Numpad9,
//             Fn => K::Function,
//             _ => return Err(anyhow::anyhow!("Unknown key: {}", key.0)),
//         };
//
//         Ok(enigo_key)
//     }
// }
