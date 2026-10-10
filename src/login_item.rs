//! Launching Spectacle when the person signs in to Windows, through the
//! per-user Run key that Task Manager's Startup apps page also manages.

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_BINARY, REG_OPTION_NON_VOLATILE, REG_ROUTINE_FLAGS,
    REG_SZ, REG_VALUE_TYPE, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW,
    RegDeleteKeyValueW, RegGetValueW, RegSetValueExW,
};
use windows::core::{PCWSTR, w};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const STARTUP_APPROVED_KEY: PCWSTR =
    w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");
const VALUE_NAME: PCWSTR = w!("Spectacle");

/// The command the Run key holds: the quoted path of this executable.
fn launch_command() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    Some(format!("\"{}\"", path.display()))
}

/// Whether Spectacle is set to launch at sign-in, and has not been disabled
/// in Task Manager.
pub fn is_enabled() -> bool {
    let Some(command) = read_string(RUN_KEY, VALUE_NAME) else {
        return false;
    };
    let Some(expected) = launch_command() else {
        return false;
    };
    if !command.trim().eq_ignore_ascii_case(&expected) {
        return false;
    }
    // Task Manager marks a disabled startup app with an odd first byte.
    !read_binary(STARTUP_APPROVED_KEY, VALUE_NAME)
        .is_some_and(|state| state.first().is_some_and(|byte| byte & 1 == 1))
}

pub fn set_enabled(enabled: bool) -> windows::core::Result<()> {
    // Clearing Task Manager's state lets the Run entry take effect again.
    delete_value(STARTUP_APPROVED_KEY, VALUE_NAME);
    if !enabled {
        delete_value(RUN_KEY, VALUE_NAME);
        return Ok(());
    }
    let command = launch_command().ok_or_else(windows::core::Error::from_thread)?;
    let data: Vec<u8> = command
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut key = HKEY::default();
    // SAFETY: `key` receives an open handle that is closed below.
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    }
    .ok()?;
    // SAFETY: `data` is a null-terminated UTF-16 string as REG_SZ requires.
    let result = unsafe { RegSetValueExW(key, VALUE_NAME, None, REG_SZ, Some(&data)) }.ok();
    // SAFETY: the key was opened above and is closed once.
    let _ = unsafe { RegCloseKey(key) };
    result
}

fn read_string(key: PCWSTR, value: PCWSTR) -> Option<String> {
    let bytes = read_value(key, value, RRF_RT_REG_SZ)?;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|unit| u16::from_le_bytes(*unit))
        .take_while(|unit| *unit != 0)
        .collect();
    Some(String::from_utf16_lossy(&units))
}

fn read_binary(key: PCWSTR, value: PCWSTR) -> Option<Vec<u8>> {
    read_value(key, value, RRF_RT_REG_BINARY)
}

fn read_value(key: PCWSTR, value: PCWSTR, flags: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    let mut size = 0u32;
    let mut kind = REG_VALUE_TYPE::default();
    // SAFETY: a null data pointer asks for the value's size.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            value,
            flags,
            Some(&mut kind),
            None,
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS || size == 0 || size > 64 * 1024 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    // SAFETY: `data` holds `size` bytes as reported above.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            value,
            flags,
            Some(&mut kind),
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_BINARY) {
        return None;
    }
    data.truncate(size as usize);
    Some(data)
}

fn delete_value(key: PCWSTR, value: PCWSTR) {
    // SAFETY: deleting a value that does not exist is harmless.
    let _ = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key, value) };
}
