//! Shortcuts and settings, stored as JSON in %APPDATA%\Spectacle.
//!
//! Shortcuts.json uses the same format as the macOS version. Files are
//! replaced atomically, and a file that cannot be read is set aside rather
//! than silently overwritten.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

use crate::action::WindowAction;
use crate::shortcut::{KeyBinding, Shortcut, default_shortcuts};

const SHORTCUTS_FILE: &str = "Shortcuts.json";
const SETTINGS_FILE: &str = "Settings.json";
/// Larger files are not Spectacle's and are left unread.
const MAXIMUM_FILE_SIZE: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Settings {
    /// Whether Spectacle shows its icon in the system tray.
    pub status_item_enabled: bool,
    /// Whether the explanation shown when hiding the tray icon is suppressed.
    pub background_alert_suppressed: bool,
    /// Executable names of the applications shortcuts are disabled for.
    pub disabled_applications: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            status_item_enabled: true,
            background_alert_suppressed: false,
            disabled_applications: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredShortcut {
    shortcut_name: String,
    shortcut_key_binding: Option<String>,
}

pub struct Storage {
    directory: PathBuf,
}

impl Storage {
    /// Storage in %APPDATA%\Spectacle.
    pub fn new() -> io::Result<Storage> {
        // SAFETY: the returned string is freed with CoTaskMemFree below.
        let path = unsafe { SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None) }
            .map_err(|error| io::Error::other(error.message()))?;
        // SAFETY: SHGetKnownFolderPath returned a valid null-terminated string.
        let roaming = unsafe { path.to_string() };
        // SAFETY: the pointer came from SHGetKnownFolderPath and is freed once.
        unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
        let roaming = roaming.map_err(|error| io::Error::other(error.to_string()))?;
        Ok(Storage::in_directory(Path::new(&roaming).join("Spectacle")))
    }

    pub fn in_directory(directory: PathBuf) -> Storage {
        Storage { directory }
    }

    /// Whether Spectacle has never saved its settings for this user.
    pub fn is_first_launch(&self) -> bool {
        !self.directory.join(SETTINGS_FILE).exists()
    }

    /// The stored shortcuts, with the default for every action the file does
    /// not mention or mentions with a binding that cannot be read.
    pub fn load_shortcuts(&self) -> Vec<Shortcut> {
        let stored: Vec<StoredShortcut> = self.read_json(SHORTCUTS_FILE).unwrap_or_default();
        default_shortcuts()
            .into_iter()
            .map(|default| {
                let Some(entry) = stored
                    .iter()
                    .find(|s| s.shortcut_name == default.action.shortcut_name())
                else {
                    return default;
                };
                match &entry.shortcut_key_binding {
                    None => Shortcut {
                        action: default.action,
                        binding: None,
                    },
                    Some(text) => match KeyBinding::parse(text) {
                        Some(binding) if binding.is_valid_shortcut() => Shortcut {
                            action: default.action,
                            binding: Some(binding),
                        },
                        _ => default,
                    },
                }
            })
            .collect()
    }

    pub fn store_shortcuts(&self, shortcuts: &[Shortcut]) -> io::Result<()> {
        let stored: Vec<StoredShortcut> = WindowAction::ALL
            .iter()
            .filter_map(|action| shortcuts.iter().find(|s| s.action == *action))
            .map(|shortcut| StoredShortcut {
                shortcut_name: shortcut.action.shortcut_name().to_owned(),
                shortcut_key_binding: shortcut.binding.and_then(|b| b.to_binding_string()),
            })
            .collect();
        self.write_json(SHORTCUTS_FILE, &stored)
    }

    pub fn load_settings(&self) -> Settings {
        self.read_json(SETTINGS_FILE).unwrap_or_default()
    }

    pub fn store_settings(&self, settings: &Settings) -> io::Result<()> {
        self.write_json(SETTINGS_FILE, settings)
    }

    fn read_json<T: for<'de> Deserialize<'de>>(&self, name: &str) -> Option<T> {
        let path = self.directory.join(name);
        let mut contents = Vec::new();
        let file = fs::File::open(&path).ok()?;
        file.take(MAXIMUM_FILE_SIZE + 1)
            .read_to_end(&mut contents)
            .ok()?;
        let parsed = if contents.len() as u64 > MAXIMUM_FILE_SIZE {
            None
        } else {
            serde_json::from_slice(&contents).ok()
        };
        if parsed.is_none() {
            // Keep the unreadable file for the person to inspect instead of
            // overwriting it with defaults.
            let _ = fs::rename(&path, path.with_extension("json.invalid"));
        }
        parsed
    }

    fn write_json<T: Serialize + ?Sized>(&self, name: &str, value: &T) -> io::Result<()> {
        fs::create_dir_all(&self.directory)?;
        let path = self.directory.join(name);
        let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
        let result = (|| {
            let mut contents = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
            contents.push(b'\n');
            let mut file = fs::File::create(&temporary)?;
            file.write_all(&contents)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut::{MOD_ALT, MOD_CONTROL};

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("spectacle-test-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_files_give_defaults() {
        let directory = TemporaryDirectory::new("missing");
        let storage = Storage::in_directory(directory.0.clone());
        assert!(storage.is_first_launch());
        assert_eq!(storage.load_shortcuts(), default_shortcuts());
        assert_eq!(storage.load_settings(), Settings::default());
    }

    #[test]
    fn shortcuts_round_trip_including_cleared_ones() {
        let directory = TemporaryDirectory::new("round-trip");
        let storage = Storage::in_directory(directory.0.clone());
        let mut shortcuts = default_shortcuts();
        shortcuts[0].binding = None;
        shortcuts[1].binding = Some(KeyBinding::new(MOD_CONTROL | MOD_ALT, 0x7B));
        storage.store_shortcuts(&shortcuts).unwrap();
        assert_eq!(storage.load_shortcuts(), shortcuts);
        let text = fs::read_to_string(directory.0.join(SHORTCUTS_FILE)).unwrap();
        assert!(text.contains("\"shortcut_name\": \"MoveToCenter\""));
        assert!(text.contains("\"shortcut_key_binding\": null"));
        assert!(text.contains("\"shortcut_key_binding\": \"ctrl+alt+f12\""));
    }

    #[test]
    fn missing_and_unreadable_entries_fall_back_to_defaults() {
        let directory = TemporaryDirectory::new("partial");
        fs::create_dir_all(&directory.0).unwrap();
        fs::write(
            directory.0.join(SHORTCUTS_FILE),
            r#"[{"shortcut_name":"MoveToCenter","shortcut_key_binding":"ctrl+alt+x"},
                {"shortcut_name":"MoveToFullscreen","shortcut_key_binding":"hyper+f"},
                {"shortcut_name":"Unknown","shortcut_key_binding":"ctrl+alt+q"}]"#,
        )
        .unwrap();
        let storage = Storage::in_directory(directory.0.clone());
        let shortcuts = storage.load_shortcuts();
        let defaults = default_shortcuts();
        assert_eq!(
            shortcuts[0].binding,
            Some(KeyBinding::new(MOD_CONTROL | MOD_ALT, u16::from(b'X')))
        );
        assert_eq!(shortcuts[1], defaults[1]);
        assert_eq!(shortcuts[2..], defaults[2..]);
    }

    #[test]
    fn corrupt_files_are_set_aside() {
        let directory = TemporaryDirectory::new("corrupt");
        fs::create_dir_all(&directory.0).unwrap();
        fs::write(directory.0.join(SETTINGS_FILE), "{ not json").unwrap();
        let storage = Storage::in_directory(directory.0.clone());
        assert_eq!(storage.load_settings(), Settings::default());
        assert!(directory.0.join("Settings.json.invalid").exists());
        assert!(!directory.0.join(SETTINGS_FILE).exists());
    }

    #[test]
    fn settings_round_trip() {
        let directory = TemporaryDirectory::new("settings");
        let storage = Storage::in_directory(directory.0.clone());
        let settings = Settings {
            status_item_enabled: false,
            background_alert_suppressed: true,
            disabled_applications: vec!["notepad.exe".into()],
        };
        storage.store_settings(&settings).unwrap();
        assert!(!storage.is_first_launch());
        assert_eq!(storage.load_settings(), settings);
    }
}
