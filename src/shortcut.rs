//! Keyboard shortcuts: their key bindings, how bindings are written in
//! Shortcuts.json and how they are shown to people.

use windows::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_CHAR, MapVirtualKeyW};

use crate::action::WindowAction;

pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;
const ALL_MODIFIERS: u32 = MOD_ALT | MOD_CONTROL | MOD_SHIFT | MOD_WIN;

const VK_F1: u16 = 0x70;
const VK_F24: u16 = 0x87;

/// A key combined with modifier keys, as passed to `RegisterHotKey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub modifiers: u32,
    pub key: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub action: WindowAction,
    /// `None` when the shortcut has been cleared.
    pub binding: Option<KeyBinding>,
}

/// Modifiers in the order Windows writes them, with their binding names.
const MODIFIER_NAMES: [(u32, &str); 4] = [
    (MOD_WIN, "win"),
    (MOD_CONTROL, "ctrl"),
    (MOD_ALT, "alt"),
    (MOD_SHIFT, "shift"),
];

/// Keys a shortcut can use: virtual-key code, binding name and, for keys
/// that are not labelled by the keyboard layout, the label shown.
const KEYS: &[(u16, &str, &str)] = &[
    (0x08, "backspace", "Backspace"),
    (0x09, "tab", "Tab"),
    (0x0D, "enter", "Enter"),
    (0x13, "pause", "Pause"),
    (0x1B, "esc", "Esc"),
    (0x20, "space", "Space"),
    (0x21, "pageup", "PgUp"),
    (0x22, "pagedown", "PgDn"),
    (0x23, "end", "End"),
    (0x24, "home", "Home"),
    (0x25, "left", "\u{2190}"),
    (0x26, "up", "\u{2191}"),
    (0x27, "right", "\u{2192}"),
    (0x28, "down", "\u{2193}"),
    (0x2C, "printscreen", "PrtSc"),
    (0x2D, "insert", "Ins"),
    (0x2E, "delete", "Del"),
    (0x60, "num0", "Num 0"),
    (0x61, "num1", "Num 1"),
    (0x62, "num2", "Num 2"),
    (0x63, "num3", "Num 3"),
    (0x64, "num4", "Num 4"),
    (0x65, "num5", "Num 5"),
    (0x66, "num6", "Num 6"),
    (0x67, "num7", "Num 7"),
    (0x68, "num8", "Num 8"),
    (0x69, "num9", "Num 9"),
    (0x6A, "nummultiply", "Num *"),
    (0x6B, "numadd", "Num +"),
    (0x6D, "numsubtract", "Num -"),
    (0x6E, "numdecimal", "Num ."),
    (0x6F, "numdivide", "Num /"),
    (0xBA, "semicolon", ""),
    (0xBB, "equal", ""),
    (0xBC, "comma", ""),
    (0xBD, "minus", ""),
    (0xBE, "period", ""),
    (0xBF, "slash", ""),
    (0xC0, "backquote", ""),
    (0xDB, "bracketleft", ""),
    (0xDC, "backslash", ""),
    (0xDD, "bracketright", ""),
    (0xDE, "quote", ""),
    (0xDF, "oem8", ""),
    (0xE2, "oem102", ""),
];

/// The US layout characters of the punctuation keys, used when the current
/// keyboard layout has no character for one.
const US_PUNCTUATION: &[(u16, char)] = &[
    (0xBA, ';'),
    (0xBB, '='),
    (0xBC, ','),
    (0xBD, '-'),
    (0xBE, '.'),
    (0xBF, '/'),
    (0xC0, '`'),
    (0xDB, '['),
    (0xDC, '\\'),
    (0xDD, ']'),
    (0xDE, '\''),
    (0xDF, '!'),
    (0xE2, '\\'),
];

impl KeyBinding {
    pub const fn new(modifiers: u32, key: u16) -> Self {
        Self { modifiers, key }
    }

    /// Parses a binding such as `win+ctrl+alt+left`. Names are case
    /// insensitive; any unknown part makes the whole binding invalid.
    pub fn parse(text: &str) -> Option<KeyBinding> {
        let mut parts: Vec<String> = text
            .split('+')
            .map(|part| part.trim().to_ascii_lowercase())
            .collect();
        let key_name = parts.pop()?;
        let key = key_code_for_name(&key_name)?;
        let mut modifiers = 0;
        for part in &parts {
            let modifier = match part.as_str() {
                "win" | "windows" | "super" => MOD_WIN,
                "ctrl" | "control" => MOD_CONTROL,
                "alt" => MOD_ALT,
                "shift" => MOD_SHIFT,
                _ => return None,
            };
            modifiers |= modifier;
        }
        Some(KeyBinding { modifiers, key })
    }

    /// The binding as written in Shortcuts.json, e.g. `win+ctrl+alt+left`.
    pub fn to_binding_string(self) -> Option<String> {
        let key = name_for_key_code(self.key)?;
        let mut parts: Vec<&str> = MODIFIER_NAMES
            .iter()
            .filter(|(modifier, _)| self.modifiers & modifier != 0)
            .map(|(_, name)| *name)
            .collect();
        parts.push(key);
        Some(parts.join("+"))
    }

    /// The keys of the binding as people see them on the keyboard, e.g.
    /// `["Win", "Ctrl", "Alt", "←"]`.
    pub fn display_keys(self, shift_label: &str) -> Vec<String> {
        let mut keys: Vec<String> = MODIFIER_NAMES
            .iter()
            .filter(|(modifier, _)| self.modifiers & modifier != 0)
            .map(|(modifier, _)| modifier_label(*modifier, shift_label).to_owned())
            .collect();
        keys.push(key_label(self.key));
        keys
    }

    /// The binding as text, e.g. `Win+Ctrl+Alt+←`.
    pub fn display_string(self, shift_label: &str) -> String {
        self.display_keys(shift_label).join("+")
    }

    /// The binding as screen readers should read it, with arrow keys named
    /// rather than drawn, e.g. `Win+Ctrl+Alt+Left`.
    pub fn spoken_string(self, shift_label: &str) -> String {
        let mut keys = self.display_keys(shift_label);
        if let Some(key) = keys.last_mut()
            && let Some((_, name, _)) = KEYS
                .iter()
                .find(|(code, _, _)| *code == self.key && (0x25..=0x28).contains(code))
        {
            let mut spoken = (*name).to_owned();
            spoken[..1].make_ascii_uppercase();
            *key = spoken;
        }
        keys.join("+")
    }

    /// Whether this binding is suitable as a system-wide shortcut. Shortcuts
    /// need Win, Ctrl or Alt so that they do not take over ordinary typing,
    /// except for the function keys which are rarely typed.
    pub fn is_valid_shortcut(self) -> bool {
        name_for_key_code(self.key).is_some()
            && self.modifiers & !ALL_MODIFIERS == 0
            && (self.modifiers & (MOD_WIN | MOD_CONTROL | MOD_ALT) != 0
                || is_function_key(self.key))
    }
}

pub fn is_function_key(key: u16) -> bool {
    (VK_F1..=VK_F24).contains(&key)
}

pub fn modifier_label(modifier: u32, shift_label: &str) -> &str {
    match modifier {
        MOD_WIN => "Win",
        MOD_CONTROL => "Ctrl",
        MOD_ALT => "Alt",
        _ => shift_label,
    }
}

fn key_code_for_name(name: &str) -> Option<u16> {
    let bytes = name.as_bytes();
    if bytes.len() == 1 && (bytes[0].is_ascii_alphabetic() || bytes[0].is_ascii_digit()) {
        return Some(u16::from(bytes[0].to_ascii_uppercase()));
    }
    if let Some(number) = name.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
        return (1..=24).contains(&number).then_some(VK_F1 + number - 1);
    }
    KEYS.iter()
        .find(|(_, key_name, _)| *key_name == name)
        .map(|(key, _, _)| *key)
}

fn name_for_key_code(key: u16) -> Option<&'static str> {
    const LETTERS_AND_DIGITS: [&str; 36] = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "b", "c", "d", "e", "f", "g", "h",
        "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
    ];
    const FUNCTION_KEYS: [&str; 24] = [
        "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12", "f13", "f14",
        "f15", "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23", "f24",
    ];
    match key {
        0x30..=0x39 => Some(LETTERS_AND_DIGITS[usize::from(key - 0x30)]),
        0x41..=0x5A => Some(LETTERS_AND_DIGITS[usize::from(key - 0x41) + 10]),
        VK_F1..=VK_F24 => Some(FUNCTION_KEYS[usize::from(key - VK_F1)]),
        _ => KEYS
            .iter()
            .find(|(code, _, _)| *code == key)
            .map(|(_, name, _)| *name),
    }
}

/// The label of a key as printed on the keyboard, following the current
/// keyboard layout for punctuation.
pub fn key_label(key: u16) -> String {
    match key {
        0x30..=0x39 | 0x41..=0x5A => char::from(key as u8).to_string(),
        VK_F1..=VK_F24 => format!("F{}", key - VK_F1 + 1),
        _ => {
            let Some((_, name, label)) = KEYS.iter().find(|(code, _, _)| *code == key) else {
                return format!("0x{key:02X}");
            };
            if !label.is_empty() {
                return (*label).to_owned();
            }
            // SAFETY: MapVirtualKeyW has no preconditions.
            let mapped = unsafe { MapVirtualKeyW(u32::from(key), MAPVK_VK_TO_CHAR) };
            // The high bit marks a dead key; its character is still the label.
            let character =
                char::from_u32(mapped & 0x7FFF_FFFF).filter(|c| !c.is_control() && *c != '\0');
            character
                .map(|c| c.to_uppercase().to_string())
                .or_else(|| {
                    US_PUNCTUATION
                        .iter()
                        .find(|(code, _)| *code == key)
                        .map(|(_, c)| c.to_string())
                })
                .unwrap_or_else(|| (*name).to_owned())
        }
    }
}

/// The shortcuts Spectacle starts with on Windows.
pub fn default_shortcuts() -> Vec<Shortcut> {
    const WIN_CTRL_ALT: u32 = MOD_WIN | MOD_CONTROL | MOD_ALT;
    const LEFT: u16 = 0x25;
    const UP: u16 = 0x26;
    const RIGHT: u16 = 0x27;
    const DOWN: u16 = 0x28;
    let binding = |action, modifiers, key| Shortcut {
        action,
        binding: Some(KeyBinding::new(modifiers, key)),
    };
    vec![
        binding(WindowAction::Center, WIN_CTRL_ALT, u16::from(b'C')),
        binding(WindowAction::Fullscreen, WIN_CTRL_ALT, u16::from(b'F')),
        binding(WindowAction::LeftHalf, WIN_CTRL_ALT, LEFT),
        binding(WindowAction::RightHalf, WIN_CTRL_ALT, RIGHT),
        binding(WindowAction::TopHalf, WIN_CTRL_ALT, UP),
        binding(WindowAction::BottomHalf, WIN_CTRL_ALT, DOWN),
        binding(WindowAction::UpperLeft, WIN_CTRL_ALT, u16::from(b'U')),
        binding(WindowAction::LowerLeft, WIN_CTRL_ALT, u16::from(b'J')),
        binding(WindowAction::UpperRight, WIN_CTRL_ALT, u16::from(b'I')),
        binding(WindowAction::LowerRight, WIN_CTRL_ALT, u16::from(b'K')),
        binding(
            WindowAction::NextDisplay,
            MOD_WIN | MOD_ALT | MOD_SHIFT,
            RIGHT,
        ),
        binding(
            WindowAction::PreviousDisplay,
            MOD_WIN | MOD_ALT | MOD_SHIFT,
            LEFT,
        ),
        binding(
            WindowAction::NextThird,
            MOD_WIN | MOD_CONTROL | MOD_SHIFT,
            RIGHT,
        ),
        binding(
            WindowAction::PreviousThird,
            MOD_WIN | MOD_CONTROL | MOD_SHIFT,
            LEFT,
        ),
        binding(
            WindowAction::Larger,
            MOD_CONTROL | MOD_ALT | MOD_SHIFT,
            RIGHT,
        ),
        binding(
            WindowAction::Smaller,
            MOD_CONTROL | MOD_ALT | MOD_SHIFT,
            LEFT,
        ),
        binding(WindowAction::Undo, WIN_CTRL_ALT, u16::from(b'Z')),
        binding(WindowAction::Redo, WIN_CTRL_ALT, u16::from(b'Y')),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes_bindings() {
        let binding = KeyBinding::parse("win+ctrl+alt+left").unwrap();
        assert_eq!(
            binding,
            KeyBinding::new(MOD_WIN | MOD_CONTROL | MOD_ALT, 0x25)
        );
        assert_eq!(binding.to_binding_string().unwrap(), "win+ctrl+alt+left");
    }

    #[test]
    fn parsing_is_case_and_space_insensitive_and_orders_modifiers() {
        let binding = KeyBinding::parse(" Shift + ALT+ Ctrl +F12 ").unwrap();
        assert_eq!(
            binding,
            KeyBinding::new(MOD_SHIFT | MOD_ALT | MOD_CONTROL, 0x7B)
        );
        assert_eq!(binding.to_binding_string().unwrap(), "ctrl+alt+shift+f12");
    }

    #[test]
    fn rejects_unknown_parts() {
        assert_eq!(KeyBinding::parse("hyper+left"), None);
        assert_eq!(KeyBinding::parse("ctrl+alt+nokey"), None);
        assert_eq!(KeyBinding::parse("ctrl+f25"), None);
        assert_eq!(KeyBinding::parse(""), None);
    }

    #[test]
    fn every_named_key_round_trips() {
        let keys = (0x30..=0x39)
            .chain(0x41..=0x5A)
            .chain(VK_F1..=VK_F24)
            .chain(KEYS.iter().map(|k| k.0));
        for key in keys {
            let binding = KeyBinding::new(MOD_CONTROL | MOD_ALT, key);
            let text = binding.to_binding_string().unwrap();
            assert_eq!(KeyBinding::parse(&text), Some(binding), "{text}");
        }
    }

    #[test]
    fn displays_bindings_in_windows_order() {
        let binding = KeyBinding::new(MOD_SHIFT | MOD_ALT | MOD_CONTROL | MOD_WIN, 0x27);
        assert_eq!(
            binding.display_string("Shift"),
            "Win+Ctrl+Alt+Shift+\u{2192}"
        );
        assert_eq!(
            KeyBinding::new(MOD_CONTROL, 0x70).display_string("Shift"),
            "Ctrl+F1"
        );
        assert_eq!(
            KeyBinding::new(MOD_ALT, 0x22).display_keys("Shift"),
            ["Alt", "PgDn"]
        );
    }

    #[test]
    fn shortcuts_need_a_modifier_other_than_shift_or_a_function_key() {
        assert!(KeyBinding::new(MOD_WIN, u16::from(b'C')).is_valid_shortcut());
        assert!(KeyBinding::new(MOD_ALT | MOD_SHIFT, u16::from(b'C')).is_valid_shortcut());
        assert!(!KeyBinding::new(MOD_SHIFT, u16::from(b'C')).is_valid_shortcut());
        assert!(!KeyBinding::new(0, u16::from(b'C')).is_valid_shortcut());
        assert!(KeyBinding::new(0, 0x7B).is_valid_shortcut());
        assert!(KeyBinding::new(MOD_SHIFT, 0x7B).is_valid_shortcut());
        assert!(!KeyBinding::new(MOD_CONTROL, 0x10).is_valid_shortcut());
    }

    #[test]
    fn defaults_cover_every_action_once_with_distinct_valid_bindings() {
        let defaults = default_shortcuts();
        assert_eq!(defaults.len(), WindowAction::ALL.len());
        for action in WindowAction::ALL {
            assert_eq!(
                defaults.iter().filter(|s| s.action == action).count(),
                1,
                "{action:?}"
            );
        }
        for (i, a) in defaults.iter().enumerate() {
            assert!(a.binding.unwrap().is_valid_shortcut());
            for b in &defaults[i + 1..] {
                assert_ne!(a.binding, b.binding, "{:?} and {:?}", a.action, b.action);
            }
        }
    }
}
