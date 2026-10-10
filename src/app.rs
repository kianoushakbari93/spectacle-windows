//! The application: a hidden window that receives the shortcuts and tray
//! icon events, and the state behind the tray menu and Settings window.
//!
//! The state lives in a thread-local and is borrowed only briefly; nothing
//! that can run a modal loop (dialogs, menus, other windows' messages) is
//! called while it is borrowed.

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::Shell::{NIN_SELECT, NINF_KEY};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GetMessageW, GetSystemMetrics, HMENU, KillTimer,
    MB_ICONERROR, MB_OK, MENU_ITEM_FLAGS, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG,
    MessageBoxW, OBJID_WINDOW, PostMessageW, PostQuitMessage, RegisterClassExW,
    RegisterWindowMessageW, SM_MENUDROPALIGNMENT, SetForegroundWindow, SetTimer, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_RIGHTBUTTON, TrackPopupMenuEx,
    TranslateMessage, WINDOW_STYLE, WINEVENT_OUTOFCONTEXT, WM_APP, WM_CLOSE, WM_CONTEXTMENU,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ENDSESSION, WM_HOTKEY, WM_NULL,
    WM_QUERYENDSESSION, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::action::WindowAction;
use crate::dialogs;
use crate::hotkeys::HotkeyManager;
use crate::l10n::{self, strings};
use crate::login_item;
use crate::position_manager::WindowPositionManager;
use crate::preferences;
use crate::screens;
use crate::shortcut::{KeyBinding, Shortcut, default_shortcuts};
use crate::storage::{Settings, Storage};
use crate::tray::TrayIcon;
use crate::window::{Application, Window};

pub const MAIN_WINDOW_CLASS: PCWSTR = w!("Spectacle.Main.7b0f2f53-3d2b-4d7e-9a43-6c3a5f8e2d11");
pub const SHOW_SETTINGS_MESSAGE: PCWSTR =
    w!("Spectacle.ShowSettings.7b0f2f53-3d2b-4d7e-9a43-6c3a5f8e2d11");

const TRAY_MESSAGE: u32 = WM_APP + 1;
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;
const DISABLE_FOR_AN_HOUR_TIMER: usize = 1;
const ONE_HOUR_IN_MILLISECONDS: u32 = 60 * 60 * 1000;

const COMMAND_ABOUT: u32 = 1;
const COMMAND_DISABLE_FOR_AN_HOUR: u32 = 2;
const COMMAND_DISABLE_FOR_APPLICATION: u32 = 3;
const COMMAND_SETTINGS: u32 = 4;
const COMMAND_QUIT: u32 = 5;
const COMMAND_FIRST_ACTION: u32 = 100;

/// Applications whose own shortcuts take precedence, so Spectacle's are
/// turned off while they are in front, as in the macOS version.
const BLACKLISTED_APPLICATIONS: &[&str] = &["photoshop.exe"];

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// What the Settings window shows.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub shortcuts: Vec<Shortcut>,
    pub unavailable: Vec<bool>,
    pub launch_at_login: bool,
    pub run_in_system_tray: bool,
    pub background_alert_suppressed: bool,
}

impl Snapshot {
    pub fn binding(&self, action: WindowAction) -> Option<KeyBinding> {
        self.shortcuts
            .iter()
            .find(|s| s.action == action)
            .and_then(|s| s.binding)
    }

    pub fn is_unavailable(&self, action: WindowAction) -> bool {
        self.unavailable
            .get(action.index())
            .copied()
            .unwrap_or(false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingProblem {
    UsedBySpectacle,
    UsedByAnotherApplication,
}

struct App {
    hwnd: HWND,
    storage: Storage,
    settings: Settings,
    hotkeys: HotkeyManager,
    positions: WindowPositionManager<Window>,
    tray: TrayIcon,
    disabled_for_an_hour: bool,
    /// The window and application in front before Spectacle's own windows
    /// or the taskbar were, which the tray menu acts on.
    last_window: Option<Window>,
    last_application: Option<Application>,
    own_process_id: u32,
    own_executable: String,
    hook: HWINEVENTHOOK,
    taskbar_created_message: u32,
    show_settings_message: u32,
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|app| app.try_borrow_mut().ok()?.as_mut().map(f))
}

/// Starts Spectacle and runs until it quits. Returns the exit code.
pub fn run(storage: Storage) -> i32 {
    let hwnd = match create_main_window() {
        Ok(hwnd) => hwnd,
        Err(error) => {
            report_error("Spectacle could not start.", &error.message());
            return 1;
        }
    };
    let first_launch = storage.is_first_launch();
    let settings = storage.load_settings();
    let shortcuts = storage.load_shortcuts();
    // SAFETY: plain registrations on this thread.
    let (taskbar_created_message, show_settings_message, hook) = unsafe {
        (
            RegisterWindowMessageW(w!("TaskbarCreated")),
            RegisterWindowMessageW(SHOW_SETTINGS_MESSAGE),
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_win_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            ),
        )
    };
    let own_executable = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
        })
        .unwrap_or_else(|| "spectacle.exe".into());
    let mut app = App {
        hwnd,
        storage,
        hotkeys: HotkeyManager::new(hwnd, shortcuts),
        positions: WindowPositionManager::default(),
        tray: TrayIcon::new(hwnd, TRAY_MESSAGE),
        disabled_for_an_hour: false,
        last_window: None,
        last_application: None,
        // SAFETY: GetCurrentProcessId has no preconditions.
        own_process_id: unsafe { GetCurrentProcessId() },
        own_executable,
        hook,
        taskbar_created_message,
        show_settings_message,
        settings,
    };
    if app.settings.status_item_enabled {
        app.tray.show();
    }
    if let Some(window) = Window::foreground() {
        app.foreground_changed(window);
    } else {
        app.update_shortcut_registration(None);
    }
    if first_launch {
        app.store_settings();
    }
    APP.with(|cell| *cell.borrow_mut() = Some(app));

    if first_launch {
        preferences::show();
    }
    let code = message_loop();
    APP.with(|cell| {
        if let Ok(mut app) = cell.try_borrow_mut() {
            *app = None;
        }
    });
    code
}

fn message_loop() -> i32 {
    let mut message = MSG::default();
    loop {
        // SAFETY: standard message loop on this thread.
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 == 0 || result.0 == -1 {
            break;
        }
        // SAFETY: as above.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    message.wParam.0 as i32
}

fn create_main_window() -> windows::core::Result<HWND> {
    // SAFETY: GetModuleHandleW(None) returns this executable's module.
    let instance = unsafe { GetModuleHandleW(None)? };
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_procedure),
        hInstance: instance.into(),
        lpszClassName: MAIN_WINDOW_CLASS,
        ..Default::default()
    };
    // SAFETY: the class structure is valid.
    if unsafe { RegisterClassExW(&class) } == 0 {
        return Err(windows::core::Error::from_thread());
    }
    // A hidden tool window: never shown, but able to receive broadcasts such
    // as TaskbarCreated, which message-only windows do not.
    // SAFETY: the class was registered above.
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            MAIN_WINDOW_CLASS,
            w!("Spectacle"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
}

unsafe extern "system" fn on_win_event(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND || object != OBJID_WINDOW.0 {
        return;
    }
    if let Some(window) = Window::from_hwnd(hwnd) {
        with_app(|app| app.foreground_changed(window));
    }
}

unsafe extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_HOTKEY => {
            if let Some(action) =
                with_app(|app| app.hotkeys.action_for_hotkey_id(wparam.0)).flatten()
            {
                perform(action, Window::foreground(), false);
            }
            LRESULT(0)
        }
        TRAY_MESSAGE => {
            let event = (lparam.0 & 0xFFFF) as u32;
            if matches!(event, WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT) {
                let x = (wparam.0 & 0xFFFF) as u16 as i16;
                let y = ((wparam.0 >> 16) & 0xFFFF) as u16 as i16;
                show_tray_menu(
                    hwnd,
                    POINT {
                        x: i32::from(x),
                        y: i32::from(y),
                    },
                );
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == DISABLE_FOR_AN_HOUR_TIMER => {
            with_app(|app| app.set_disabled_for_an_hour(false));
            LRESULT(0)
        }
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE | WM_DPICHANGED => {
            with_app(|app| app.tray.refresh());
            // SAFETY: default handling.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_QUERYENDSESSION => LRESULT(1),
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                shut_down(hwnd);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            shut_down(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => {
            let registered =
                with_app(|app| (app.taskbar_created_message, app.show_settings_message));
            if let Some((taskbar_created, show_settings)) = registered {
                if message == taskbar_created {
                    with_app(|app| {
                        if app.settings.status_item_enabled {
                            app.tray.show();
                        }
                    });
                    return LRESULT(0);
                }
                if message == show_settings {
                    preferences::show();
                    return LRESULT(0);
                }
            }
            // SAFETY: default handling for every other message.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

/// Removes the tray icon, unregisters the shortcuts and ends the message loop.
fn shut_down(hwnd: HWND) {
    preferences::close();
    with_app(|app| {
        app.tray.hide();
        app.hotkeys.set_enabled(false);
        if !app.hook.is_invalid() {
            // SAFETY: the hook was installed by this thread.
            let _ = unsafe { UnhookWinEvent(app.hook) };
            app.hook = HWINEVENTHOOK::default();
        }
    });
    // SAFETY: destroys this thread's window, which posts WM_QUIT.
    let _ = unsafe { DestroyWindow(hwnd) };
}

/// Performs a window action, letting the person know when nothing happened.
fn perform(action: WindowAction, window: Option<Window>, from_menu: bool) {
    let window = window.filter(|window| window.is_manageable());
    let application = match action {
        WindowAction::Undo | WindowAction::Redo => {
            if from_menu {
                with_app(|app| app.last_application.clone()).flatten()
            } else {
                Window::foreground().and_then(|window| window.application())
            }
        }
        _ => window.and_then(|window| window.application()),
    };
    let key = application
        .map(|application| application.key())
        .unwrap_or_default();
    let screens = screens::screens();
    let performed =
        with_app(|app| app.positions.perform(action, window, &key, &screens)).unwrap_or(false);
    #[cfg(debug_assertions)]
    log(&format!(
        "{action:?} on {:?}: {}",
        window.map(|window| window.hwnd()),
        if performed {
            "performed"
        } else {
            "nothing to do"
        }
    ));
    if !performed {
        // SAFETY: MessageBeep has no preconditions.
        let _ = unsafe { MessageBeep(MB_OK) };
    }
}

struct MenuModel {
    shortcuts: Vec<Shortcut>,
    disabled_for_an_hour: bool,
    application: Option<(String, bool)>,
}

fn show_tray_menu(hwnd: HWND, anchor: POINT) {
    let Some(model) = with_app(|app| app.menu_model()) else {
        return;
    };
    let Ok(menu) = build_menu(&model) else {
        return;
    };
    // SAFETY: GetSystemMetrics has no preconditions.
    let horizontal = if unsafe { GetSystemMetrics(SM_MENUDROPALIGNMENT) } != 0 {
        TPM_RIGHTALIGN
    } else {
        TPM_LEFTALIGN
    };
    // SAFETY: the menu belongs to this thread; Microsoft requires the owner to
    // be in front while a notification area menu is open, and a message to be
    // posted afterwards so it closes properly.
    let command = unsafe {
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenuEx(
            menu.0,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | horizontal).0,
            anchor.x,
            anchor.y,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        command.0 as u32
    };
    drop(menu);
    handle_menu_command(hwnd, command);
}

/// A popup menu, destroyed when dropped.
struct Menu(HMENU);

impl Drop for Menu {
    fn drop(&mut self) {
        // SAFETY: the menu was created by build_menu and is destroyed once.
        let _ = unsafe { DestroyMenu(self.0) };
    }
}

fn build_menu(model: &MenuModel) -> windows::core::Result<Menu> {
    let strings = strings();
    // SAFETY: menu creation on this thread; strings outlive each call.
    unsafe {
        let menu = Menu(CreatePopupMenu()?);
        let append = |menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, text: &str| {
            let text = HSTRING::from(text);
            AppendMenuW(menu, flags, id, &text)
        };
        let separator = |menu: HMENU| AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

        append(
            menu.0,
            MF_STRING,
            COMMAND_ABOUT as usize,
            strings.menu_about,
        )?;
        separator(menu.0)?;
        let disable = CreatePopupMenu()?;
        let checked = |on: bool| if on { MF_CHECKED } else { MENU_ITEM_FLAGS(0) };
        append(
            disable,
            MF_STRING | checked(model.disabled_for_an_hour),
            COMMAND_DISABLE_FOR_AN_HOUR as usize,
            strings.menu_disable_for_an_hour,
        )?;
        if let Some((name, disabled)) = &model.application {
            // Menus treat & as a mnemonic marker.
            let title = l10n::format(
                strings.menu_disable_for_application,
                &name.replace('&', "&&"),
            );
            append(
                disable,
                MF_STRING | checked(*disabled),
                COMMAND_DISABLE_FOR_APPLICATION as usize,
                &title,
            )?;
        }
        append(menu.0, MF_POPUP, disable.0 as usize, strings.menu_disable)?;
        separator(menu.0)?;
        append(
            menu.0,
            MF_STRING,
            COMMAND_SETTINGS as usize,
            strings.menu_settings,
        )?;
        separator(menu.0)?;

        let groups: [&[WindowAction]; 7] = [
            &[WindowAction::Center, WindowAction::Fullscreen],
            &[
                WindowAction::LeftHalf,
                WindowAction::RightHalf,
                WindowAction::TopHalf,
                WindowAction::BottomHalf,
            ],
            &[
                WindowAction::UpperLeft,
                WindowAction::LowerLeft,
                WindowAction::UpperRight,
                WindowAction::LowerRight,
            ],
            &[WindowAction::NextDisplay, WindowAction::PreviousDisplay],
            &[WindowAction::NextThird, WindowAction::PreviousThird],
            &[WindowAction::Larger, WindowAction::Smaller],
            &[WindowAction::Undo, WindowAction::Redo],
        ];
        for group in groups {
            for action in group {
                let binding = model
                    .shortcuts
                    .iter()
                    .find(|s| s.action == *action)
                    .and_then(|s| s.binding);
                let title = match binding {
                    Some(binding) => format!(
                        "{}\t{}",
                        strings.action(*action),
                        binding.display_string(strings.shift_key)
                    ),
                    None => strings.action(*action).to_owned(),
                };
                append(
                    menu.0,
                    MF_STRING,
                    (COMMAND_FIRST_ACTION as usize) + action.index(),
                    &title,
                )?;
            }
            separator(menu.0)?;
        }
        append(menu.0, MF_STRING, COMMAND_QUIT as usize, strings.menu_quit)?;
        Ok(menu)
    }
}

fn handle_menu_command(hwnd: HWND, command: u32) {
    match command {
        0 => {}
        COMMAND_ABOUT => dialogs::about(),
        COMMAND_SETTINGS => preferences::show(),
        COMMAND_QUIT => shut_down(hwnd),
        COMMAND_DISABLE_FOR_AN_HOUR => {
            with_app(|app| {
                let disabled = !app.disabled_for_an_hour;
                app.set_disabled_for_an_hour(disabled);
            });
        }
        COMMAND_DISABLE_FOR_APPLICATION => {
            with_app(|app| app.toggle_disabled_for_last_application());
        }
        _ => {
            let index = (command - COMMAND_FIRST_ACTION) as usize;
            if let Some(action) = WindowAction::ALL.get(index).copied() {
                let window = with_app(|app| app.last_window).flatten();
                perform(action, window, true);
                if let Some(window) = window {
                    // Hand the keyboard back to the window acted on.
                    // SAFETY: plain window calls with a possibly stale handle.
                    unsafe {
                        let _ = SetForegroundWindow(window.hwnd());
                        let _ = SetFocus(Some(window.hwnd()));
                    }
                }
            }
        }
    }
}

impl App {
    fn menu_model(&self) -> MenuModel {
        let application = self.last_application.as_ref().and_then(|application| {
            let executable = application.executable_name().to_lowercase();
            (!self.is_blacklisted(&executable))
                .then(|| (application.display_name(), self.is_disabled(&executable)))
        });
        MenuModel {
            shortcuts: self.hotkeys.shortcuts().to_vec(),
            disabled_for_an_hour: self.disabled_for_an_hour,
            application,
        }
    }

    fn is_blacklisted(&self, executable: &str) -> bool {
        executable == self.own_executable || BLACKLISTED_APPLICATIONS.contains(&executable)
    }

    fn is_disabled(&self, executable: &str) -> bool {
        self.settings
            .disabled_applications
            .iter()
            .any(|disabled| disabled.eq_ignore_ascii_case(executable))
    }

    fn foreground_changed(&mut self, window: Window) {
        let application = window.application();
        let own = application
            .as_ref()
            .is_some_and(|application| application.process_id == self.own_process_id)
            || window.process_id() == self.own_process_id;
        if !own && !window.is_shell_window() {
            self.last_window = Some(window).filter(|window| window.is_manageable());
            self.last_application = application.clone();
        }
        let foreground = if own {
            Some(self.own_executable.clone())
        } else {
            application.map(|a| a.executable_name().to_lowercase())
        };
        self.update_shortcut_registration(foreground.as_deref());
        self.positions.prune();
    }

    /// Registers the shortcuts unless they are disabled for an hour or for
    /// the application in front.
    fn update_shortcut_registration(&mut self, foreground_executable: Option<&str>) {
        let suppressed = foreground_executable.is_some_and(|executable| {
            self.is_blacklisted(executable) || self.is_disabled(executable)
        });
        self.hotkeys
            .set_enabled(!self.disabled_for_an_hour && !suppressed);
        #[cfg(debug_assertions)]
        log(&format!(
            "foreground {foreground_executable:?}: shortcuts {}",
            if self.hotkeys.is_enabled() {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }

    fn refresh_shortcut_registration(&mut self) {
        let foreground = Window::foreground().and_then(|window| {
            if window.process_id() == self.own_process_id {
                Some(self.own_executable.clone())
            } else {
                window
                    .application()
                    .map(|application| application.executable_name().to_lowercase())
            }
        });
        self.update_shortcut_registration(foreground.as_deref());
    }

    fn set_disabled_for_an_hour(&mut self, disabled: bool) {
        self.disabled_for_an_hour = disabled;
        // SAFETY: timers on this thread's window.
        unsafe {
            if disabled {
                SetTimer(
                    Some(self.hwnd),
                    DISABLE_FOR_AN_HOUR_TIMER,
                    ONE_HOUR_IN_MILLISECONDS,
                    None,
                );
            } else {
                let _ = KillTimer(Some(self.hwnd), DISABLE_FOR_AN_HOUR_TIMER);
            }
        }
        self.refresh_shortcut_registration();
    }

    fn toggle_disabled_for_last_application(&mut self) {
        let Some(application) = &self.last_application else {
            // SAFETY: MessageBeep has no preconditions.
            let _ = unsafe { MessageBeep(MB_OK) };
            return;
        };
        let executable = application.executable_name().to_lowercase();
        if self.is_disabled(&executable) {
            self.settings
                .disabled_applications
                .retain(|disabled| !disabled.eq_ignore_ascii_case(&executable));
        } else {
            self.settings.disabled_applications.push(executable);
        }
        self.store_settings();
        self.refresh_shortcut_registration();
    }

    fn store_settings(&self) {
        if let Err(error) = self.storage.store_settings(&self.settings) {
            log(&format!("Unable to store the settings: {error}"));
        }
    }

    fn store_shortcuts(&self) {
        if let Err(error) = self.storage.store_shortcuts(self.hotkeys.shortcuts()) {
            log(&format!("Unable to store the shortcuts: {error}"));
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            shortcuts: self.hotkeys.shortcuts().to_vec(),
            unavailable: WindowAction::ALL
                .iter()
                .map(|action| self.hotkeys.is_unavailable(*action))
                .collect(),
            launch_at_login: login_item::is_enabled(),
            run_in_system_tray: self.settings.status_item_enabled,
            background_alert_suppressed: self.settings.background_alert_suppressed,
        }
    }
}

// The interface the Settings window uses. None of these run modal loops.

pub fn snapshot() -> Option<Snapshot> {
    with_app(|app| app.snapshot())
}

/// Checks a newly recorded binding. `Ok(false)` means it is the binding the
/// action already has.
pub fn validate_binding(
    action: WindowAction,
    binding: KeyBinding,
) -> Option<Result<bool, BindingProblem>> {
    with_app(|app| {
        if app.hotkeys.binding(action) == Some(binding) {
            return Ok(false);
        }
        if app.hotkeys.action_using(binding, action).is_some() {
            return Err(BindingProblem::UsedBySpectacle);
        }
        if app.hotkeys.is_enabled() {
            // Spectacle's own registrations would get in the way of the check;
            // they are normally off while its own window is in front.
            app.hotkeys.set_enabled(false);
            let available = app.hotkeys.is_available_system_wide(binding);
            app.refresh_shortcut_registration();
            return if available {
                Ok(true)
            } else {
                Err(BindingProblem::UsedByAnotherApplication)
            };
        }
        if app.hotkeys.is_available_system_wide(binding) {
            Ok(true)
        } else {
            Err(BindingProblem::UsedByAnotherApplication)
        }
    })
}

pub fn set_shortcut(action: WindowAction, binding: Option<KeyBinding>) {
    with_app(|app| {
        app.hotkeys.update(action, binding);
        app.store_shortcuts();
    });
}

pub fn restore_default_shortcuts() {
    with_app(|app| {
        app.hotkeys.replace_all(&default_shortcuts());
        app.store_shortcuts();
    });
}

pub fn set_launch_at_login(enabled: bool) {
    if let Err(error) = login_item::set_enabled(enabled) {
        log(&format!(
            "Unable to change the sign-in setting: {}",
            error.message()
        ));
    }
}

pub fn set_run_in_system_tray(enabled: bool) {
    with_app(|app| {
        app.settings.status_item_enabled = enabled;
        if enabled {
            app.tray.show();
        } else {
            app.tray.hide();
        }
        app.store_settings();
    });
}

pub fn set_background_alert_suppressed(suppressed: bool) {
    with_app(|app| {
        app.settings.background_alert_suppressed = suppressed;
        app.store_settings();
    });
}

/// Shows an error that keeps Spectacle from doing what was asked.
pub fn report_error(title: &str, detail: &str) {
    log(&format!("{title} {detail}"));
    let text = HSTRING::from(format!("{title}\n\n{detail}"));
    // SAFETY: the strings outlive the modal call.
    unsafe { MessageBoxW(None, &text, w!("Spectacle"), MB_OK | MB_ICONERROR) };
}

/// Writes a diagnostic message for debuggers, such as DebugView.
pub fn log(message: &str) {
    let text = HSTRING::from(format!("Spectacle: {message}\n"));
    // SAFETY: the string is null terminated.
    unsafe { windows::Win32::System::Diagnostics::Debug::OutputDebugStringW(&text) };
    // Development builds also keep a log file, which is easier to follow.
    #[cfg(debug_assertions)]
    {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("spectacle-debug.log"))
        {
            let _ = writeln!(file, "{message}");
        }
    }
}
