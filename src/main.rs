//! Spectacle for Windows: move and resize windows with keyboard shortcuts.

#![windows_subsystem = "windows"]

mod accessibility;
mod action;
mod app;
mod calculations;
mod dialogs;
mod geometry;
mod graphics;
mod history;
mod hotkeys;
mod l10n;
mod login_item;
mod movers;
mod paint;
mod position_manager;
mod preferences;
mod screens;
mod shortcut;
mod storage;
mod theme;
mod tray;
mod window;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, LPARAM, WPARAM,
};
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
};
use windows::Win32::System::Recovery::{
    REGISTER_APPLICATION_RESTART_FLAGS, RegisterApplicationRestart,
};
use windows::Win32::System::SystemServices::{
    PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY, PROCESS_MITIGATION_IMAGE_LOAD_POLICY,
};
use windows::Win32::System::Threading::{
    CreateMutexW, ProcessExtensionPointDisablePolicy, ProcessImageLoadPolicy,
    SetProcessMitigationPolicy,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, FindWindowW, GetWindowThreadProcessId, PostMessageW,
    RegisterWindowMessageW,
};
use windows::core::{PCWSTR, w};

/// The version shown to people.
pub const VERSION: &str = "3.0.1";

/// Held while Spectacle runs, so a second launch can find the first. The
/// installer checks the same name to close Spectacle before updating it.
const INSTANCE_MUTEX: PCWSTR = w!("Local\\Spectacle.Instance.7b0f2f53-3d2b-4d7e-9a43-6c3a5f8e2d11");
const INSTALLER_MUTEX: PCWSTR =
    w!("Global\\Spectacle.Instance.7b0f2f53-3d2b-4d7e-9a43-6c3a5f8e2d11");

/// Restart Manager restarts Spectacle with this argument after an update.
const RESTART_ARGUMENT: PCWSTR = w!("--restarted");

fn main() {
    harden_process();

    // SAFETY: plain object creation; the handles live until the process ends.
    let instance = unsafe { CreateMutexW(None, false, INSTANCE_MUTEX) };
    // SAFETY: GetLastError reads the result of the call just made.
    if instance.is_err() || unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        show_running_instance();
        return;
    }
    // Owned by the person's session; failing to create it only affects the
    // installer's detection in other sessions.
    // SAFETY: as above.
    let installer = unsafe { CreateMutexW(None, false, INSTALLER_MUTEX) }.ok();

    // SAFETY: initialises COM for this UI thread, balanced below.
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    theme::allow_dark_menus();
    // Let Restart Manager reopen Spectacle when an update closes it.
    // SAFETY: the argument is a valid string.
    let _ = unsafe {
        RegisterApplicationRestart(
            RESTART_ARGUMENT,
            REGISTER_APPLICATION_RESTART_FLAGS(1 | 2 | 8),
        )
    };

    let code = match storage::Storage::new() {
        Ok(storage) => app::run(storage),
        Err(error) => {
            app::report_error(
                "Spectacle could not find where to keep its settings.",
                &error.to_string(),
            );
            1
        }
    };

    if com.is_ok() {
        // SAFETY: balances the successful CoInitializeEx above.
        unsafe { CoUninitialize() };
    }
    for handle in [instance.ok(), installer].into_iter().flatten() {
        close(handle);
    }
    std::process::exit(code);
}

fn close(handle: HANDLE) {
    // SAFETY: the handle was opened by this process and is closed once.
    let _ = unsafe { CloseHandle(handle) };
}

/// Opts in to exploit mitigations that Spectacle does not need to opt out of:
/// DLLs load from System32 only, never from remote or low integrity
/// locations, and legacy extension points cannot inject code.
fn harden_process() {
    // SAFETY: these calls only restrict the process.
    unsafe {
        let _ = SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
        let mut image_load = PROCESS_MITIGATION_IMAGE_LOAD_POLICY::default();
        // NoRemoteImages, NoLowMandatoryLabelImages and PreferSystem32Images.
        image_load.Anonymous.Flags = 0b1 | 0b10 | 0b100;
        let _ = SetProcessMitigationPolicy(
            ProcessImageLoadPolicy,
            (&image_load as *const PROCESS_MITIGATION_IMAGE_LOAD_POLICY).cast(),
            size_of::<PROCESS_MITIGATION_IMAGE_LOAD_POLICY>(),
        );
        let mut extension_points = PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY::default();
        extension_points.Anonymous.Flags = 1;
        let _ = SetProcessMitigationPolicy(
            ProcessExtensionPointDisablePolicy,
            (&extension_points as *const PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY).cast(),
            size_of::<PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY>(),
        );
    }
}

/// Asks the Spectacle already running to show its settings, as reopening an
/// app does on macOS.
fn show_running_instance() {
    // SAFETY: plain window lookups and a posted message.
    unsafe {
        let Ok(hwnd) = FindWindowW(app::MAIN_WINDOW_CLASS, None) else {
            return;
        };
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id != 0 {
            let _ = AllowSetForegroundWindow(process_id);
        }
        let message = RegisterWindowMessageW(app::SHOW_SETTINGS_MESSAGE);
        let _ = PostMessageW(Some(hwnd), message, WPARAM(0), LPARAM(0));
    }
}
