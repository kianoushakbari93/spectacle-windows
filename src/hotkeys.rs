//! System-wide keyboard shortcuts, registered with `RegisterHotKey`.
//!
//! RegisterHotKey delivers WM_HOTKEY to Spectacle's window only for its own
//! key combinations, so Spectacle never sees any other keystrokes.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};

use crate::action::WindowAction;
use crate::shortcut::{KeyBinding, Shortcut};

/// Hotkey identifiers are the action's index plus this base.
const FIRST_HOTKEY_ID: i32 = 1;
/// Identifier used to check whether a key combination is free.
const PROBE_HOTKEY_ID: i32 = 0xBFFF;

pub struct HotkeyManager {
    hwnd: HWND,
    shortcuts: Vec<Shortcut>,
    registered: Vec<bool>,
    failed: Vec<bool>,
    enabled: bool,
}

impl HotkeyManager {
    /// Takes over `shortcuts`, one per action, without registering them yet.
    pub fn new(hwnd: HWND, shortcuts: Vec<Shortcut>) -> HotkeyManager {
        let shortcuts: Vec<Shortcut> = WindowAction::ALL
            .iter()
            .map(|action| {
                shortcuts
                    .iter()
                    .find(|shortcut| shortcut.action == *action)
                    .copied()
                    .unwrap_or(Shortcut {
                        action: *action,
                        binding: None,
                    })
            })
            .collect();
        let count = shortcuts.len();
        HotkeyManager {
            hwnd,
            shortcuts,
            registered: vec![false; count],
            failed: vec![false; count],
            enabled: false,
        }
    }

    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.shortcuts
    }

    pub fn binding(&self, action: WindowAction) -> Option<KeyBinding> {
        self.shortcuts[action.index()].binding
    }

    /// Whether the shortcut for `action` could not be registered because
    /// another application already uses its key combination.
    pub fn is_unavailable(&self, action: WindowAction) -> bool {
        self.failed[action.index()]
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Registers every shortcut, or unregisters them all.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        for index in 0..self.shortcuts.len() {
            if enabled {
                self.register(index);
            } else {
                self.unregister(index);
            }
        }
    }

    /// Changes the binding of a shortcut, registering the new binding when
    /// shortcuts are enabled.
    pub fn update(&mut self, action: WindowAction, binding: Option<KeyBinding>) {
        let index = action.index();
        self.unregister(index);
        self.shortcuts[index].binding = binding;
        self.failed[index] = false;
        if self.enabled {
            self.register(index);
        }
    }

    pub fn replace_all(&mut self, shortcuts: &[Shortcut]) {
        for shortcut in shortcuts {
            self.update(shortcut.action, shortcut.binding);
        }
    }

    /// The action of a WM_HOTKEY identifier.
    pub fn action_for_hotkey_id(&self, id: usize) -> Option<WindowAction> {
        let index = id.checked_sub(FIRST_HOTKEY_ID as usize)?;
        WindowAction::ALL.get(index).copied()
    }

    /// The other action already using `binding`, if any.
    pub fn action_using(&self, binding: KeyBinding, except: WindowAction) -> Option<WindowAction> {
        self.shortcuts
            .iter()
            .find(|shortcut| shortcut.action != except && shortcut.binding == Some(binding))
            .map(|shortcut| shortcut.action)
    }

    /// Whether no other application or Windows itself uses `binding`.
    pub fn is_available_system_wide(&self, binding: KeyBinding) -> bool {
        if register_hotkey(self.hwnd, PROBE_HOTKEY_ID, binding) {
            // SAFETY: the hotkey was registered on this thread just above.
            let _ = unsafe { UnregisterHotKey(Some(self.hwnd), PROBE_HOTKEY_ID) };
            return true;
        }
        false
    }

    fn register(&mut self, index: usize) {
        if self.registered[index] {
            return;
        }
        let Some(binding) = self.shortcuts[index].binding else {
            self.failed[index] = false;
            return;
        };
        let registered = register_hotkey(self.hwnd, FIRST_HOTKEY_ID + index as i32, binding);
        self.registered[index] = registered;
        self.failed[index] = !registered;
    }

    fn unregister(&mut self, index: usize) {
        if self.registered[index] {
            // SAFETY: the hotkey was registered by this manager on this thread.
            let _ = unsafe { UnregisterHotKey(Some(self.hwnd), FIRST_HOTKEY_ID + index as i32) };
            self.registered[index] = false;
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.set_enabled(false);
    }
}

fn register_hotkey(hwnd: HWND, id: i32, binding: KeyBinding) -> bool {
    let modifiers = HOT_KEY_MODIFIERS(binding.modifiers) | MOD_NOREPEAT;
    // SAFETY: RegisterHotKey validates its arguments and reports failure.
    unsafe { RegisterHotKey(Some(hwnd), id, modifiers, u32::from(binding.key)) }.is_ok()
}
