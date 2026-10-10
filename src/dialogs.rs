//! Spectacle's dialogs: About, and the alerts and confirmations of the
//! Settings window.
//!
//! They are drawn like the Settings window, in the Windows 11 content dialog
//! style, so that they follow the light, dark and high contrast themes. Each
//! dialog is modal: `show` runs its own message loop until it is answered.

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, ID2D1RenderTarget,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteTextFormat,
};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, MonitorFromWindow,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{LoadIconWithScaleDown, WM_MOUSELEAVE};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, IsWindowEnabled, ReleaseCapture, SetCapture, TME_LEAVE,
    TRACKMOUSEEVENT, TrackMouseEvent, VIRTUAL_KEY, VK_CONTROL, VK_ESCAPE, VK_MENU, VK_RETURN,
    VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, GetWindowRect, HICON, IDC_ARROW, IsWindow,
    LoadCursorW, MSG, PostQuitMessage, RegisterClassExW, SW_SHOWNORMAL, SWP_NOACTIVATE,
    SWP_NOZORDER, SetForegroundWindow, SetWindowPos, ShowWindow, TranslateMessage, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_ACTIVATE, WM_CAPTURECHANGED, WM_CLOSE, WM_DESTROY, WM_DPICHANGED,
    WM_DWMCOLORIZATIONCOLORCHANGED, WM_ERASEBKGND, WM_GETDPISCALEDSIZE, WM_GETOBJECT, WM_KEYDOWN,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETTINGCHANGE,
    WM_SYSCOLORCHANGE, WM_THEMECHANGED, WNDCLASSEXW, WS_CAPTION, WS_EX_DLGMODALFRAME, WS_POPUP,
    WS_SYSMENU,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::accessibility;
use crate::graphics::{Graphics, rect_f};
use crate::l10n::{self, strings};
use crate::paint::{self, Bounds, CONTROL_RADIUS, Canvas, Painter, WindowIcons};
use crate::preferences::{AccessibleElement, AccessibleKind};
use crate::theme::Palette;

/// The application icon's resource identifier, as MAKEINTRESOURCE(1).
pub const APP_ICON_RESOURCE: PCWSTR = PCWSTR(std::ptr::without_provenance(1));

pub const CLASS_NAME: PCWSTR = w!("Spectacle.Dialog");
const DIALOG_STYLE: WINDOW_STYLE = WINDOW_STYLE(WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0);
const DIALOG_EX_STYLE: WINDOW_EX_STYLE = WS_EX_DLGMODALFRAME;

// Layout, in device independent pixels, after the Windows 11 content dialog.
const WIDTH: f32 = 440.0;
const PADDING: f32 = 24.0;
const ICON_SIZE: f32 = 48.0;
const GLYPH_SIZE: f32 = 32.0;
const ICON_GAP: f32 = 16.0;
const TITLE_SIZE: f32 = 20.0;
const TITLE_GAP: f32 = 12.0;
const BODY_SIZE: f32 = 14.0;
const CHECKBOX_GAP: f32 = 16.0;
const CHECKBOX_SIZE: f32 = 20.0;
const CHECKBOX_LABEL_GAP: f32 = 8.0;
const CONTROL_HEIGHT: f32 = 32.0;
const BUTTON_GAP: f32 = 8.0;
const FOOTER_HEIGHT: f32 = 80.0;

// Segoe Fluent Icons glyphs.
const GLYPH_WARNING: &str = "\u{E7BA}";
const GLYPH_INFO: &str = "\u{E946}";

#[derive(Clone, Copy, PartialEq)]
enum DialogIcon {
    App,
    Warning,
    Information,
}

/// The parts of a dialog, in the order assistive technologies read them.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Title,
    Content,
    Verification,
    Ok,
    Cancel,
}

struct Spec<'a> {
    owner: Option<HWND>,
    caption: &'a str,
    title: &'a str,
    content: &'a str,
    icon: DialogIcon,
    cancel: bool,
    verification: Option<&'a str>,
}

#[derive(Default)]
struct Layout {
    scale: f32,
    client_width: f32,
    icon: Bounds,
    title: Bounds,
    content: Bounds,
    checkbox: Bounds,
    checkbox_label: Bounds,
    footer: Bounds,
    ok: Bounds,
    cancel: Bounds,
}

struct Formats {
    title: IDWriteTextFormat,
    body: IDWriteTextFormat,
    label: IDWriteTextFormat,
    button: IDWriteTextFormat,
    check: IDWriteTextFormat,
    glyph: IDWriteTextFormat,
}

impl Formats {
    fn new(graphics: &Graphics, scale: f32) -> windows::core::Result<Formats> {
        Ok(Formats {
            title: graphics.paragraph_format(TITLE_SIZE * scale, DWRITE_FONT_WEIGHT_SEMI_BOLD)?,
            body: graphics.paragraph_format(BODY_SIZE * scale, DWRITE_FONT_WEIGHT_NORMAL)?,
            label: graphics.text_format(
                BODY_SIZE * scale,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?,
            button: graphics.text_format(
                BODY_SIZE * scale,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?,
            check: graphics.icon_format(12.0 * scale)?,
            glyph: graphics.icon_format(GLYPH_SIZE * scale)?,
        })
    }
}

/// The text a dialog shows, kept for laying it out again at another scale.
struct Text {
    title: String,
    content: String,
    verification: Option<String>,
    cancel: bool,
}

struct Dialog {
    hwnd: HWND,
    owner: Option<HWND>,
    text: Text,
    icon: DialogIcon,
    checked: bool,
    graphics: Graphics,
    canvas: Option<Canvas>,
    palette: Palette,
    formats: Formats,
    layout: Layout,
    app_icon: Option<HICON>,
    icons: WindowIcons,
    hover: Option<Part>,
    pressed: Option<Part>,
    focus: Part,
    show_focus: bool,
    tracking_mouse: bool,
    /// Set once the dialog is answered: whether OK was chosen.
    answer: Option<bool>,
}

thread_local! {
    /// Open dialogs; a dialog can open while another one is showing.
    static DIALOGS: RefCell<Vec<Dialog>> = const { RefCell::new(Vec::new()) };
}

fn with_dialog<R>(hwnd: HWND, f: impl FnOnce(&mut Dialog) -> R) -> Option<R> {
    DIALOGS.with(|dialogs| {
        let mut dialogs = dialogs.try_borrow_mut().ok()?;
        dialogs.iter_mut().find(|dialog| dialog.hwnd == hwnd).map(f)
    })
}

/// Brings forward the dialog open for `owner`, if any. Returns whether one is.
pub fn activate_dialog_of(owner: HWND) -> bool {
    let dialog = DIALOGS.with(|dialogs| {
        let dialogs = dialogs.try_borrow().ok()?;
        dialogs
            .iter()
            .rev()
            .find(|dialog| dialog.owner == Some(owner))
            .map(|dialog| dialog.hwnd)
    });
    if let Some(hwnd) = dialog {
        // SAFETY: the dialog belongs to this thread.
        let _ = unsafe { SetForegroundWindow(hwnd) };
        return true;
    }
    false
}

pub fn confirm_restore_defaults(owner: HWND) -> bool {
    let strings = strings();
    show(Spec {
        owner: Some(owner),
        caption: "Spectacle",
        title: strings.alert_restore_defaults_message,
        content: strings.alert_restore_defaults_informative,
        icon: DialogIcon::Warning,
        cancel: true,
        verification: None,
    })
    .0
}

/// Returns whether to run in the background and whether to stop asking.
pub fn confirm_running_in_background(owner: HWND) -> (bool, bool) {
    let strings = strings();
    show(Spec {
        owner: Some(owner),
        caption: "Spectacle",
        title: strings.alert_running_in_background_message,
        content: strings.alert_running_in_background_informative,
        icon: DialogIcon::Information,
        cancel: true,
        verification: Some(strings.do_not_show_again),
    })
}

pub fn shortcut_in_use(owner: HWND, shortcut: &str, used_by_another_application: bool) {
    let strings = strings();
    let informative = if used_by_another_application {
        strings.alert_system_shortcut_informative
    } else {
        strings.alert_shortcut_in_use_informative
    };
    show(Spec {
        owner: Some(owner),
        caption: "Spectacle",
        title: &l10n::format(strings.alert_shortcut_in_use_message, shortcut),
        content: &l10n::format(informative, shortcut),
        icon: DialogIcon::Warning,
        cancel: false,
        verification: None,
    });
}

pub fn about() {
    // One About dialog is enough.
    let open = DIALOGS.with(|dialogs| {
        let dialogs = dialogs.try_borrow().ok()?;
        dialogs
            .iter()
            .find(|dialog| dialog.icon == DialogIcon::App)
            .map(|dialog| dialog.hwnd)
    });
    if let Some(hwnd) = open {
        // SAFETY: the dialog belongs to this thread.
        let _ = unsafe { SetForegroundWindow(hwnd) };
        return;
    }
    let strings = strings();
    let content = format!(
        "{}\n\nCopyright \u{00A9} 2017 Eric Czarny\nCopyright \u{00A9} 2026 Kianoush Akbari\n\n{}",
        strings.about_description, strings.about_license
    );
    show(Spec {
        owner: None,
        caption: strings.menu_about,
        title: &format!("Spectacle {}", crate::VERSION),
        content: &content,
        icon: DialogIcon::App,
        cancel: false,
        verification: None,
    });
}

/// Shows a dialog and waits for its answer: whether OK was chosen, and
/// whether its check box was ticked.
fn show(spec: Spec) -> (bool, bool) {
    let owner = spec.owner.filter(|owner| {
        // SAFETY: plain window queries.
        unsafe { IsWindow(Some(*owner)).as_bool() && IsWindowEnabled(*owner).as_bool() }
    });
    let hwnd = match create(&Spec { owner, ..spec }) {
        Ok(hwnd) => hwnd,
        Err(error) => {
            crate::app::log(&format!("Unable to show a dialog: {}", error.message()));
            return (false, false);
        }
    };
    // SAFETY: modal behaviour: the owner stays disabled while the dialog shows.
    unsafe {
        if let Some(owner) = owner {
            let _ = EnableWindow(owner, false);
        }
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
    }
    if let Some(index) = with_dialog(hwnd, |dialog| dialog.focus_index()).flatten() {
        accessibility::focus_changed(hwnd, index);
    }

    let mut message = MSG::default();
    let mut quit = None;
    while with_dialog(hwnd, |dialog| dialog.answer.is_none()).unwrap_or(false) {
        // SAFETY: a modal message loop on this thread.
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 == 0 || result.0 == -1 {
            quit = Some(message.wParam.0 as i32);
            break;
        }
        // SAFETY: as above.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    let answer = with_dialog(hwnd, |dialog| {
        (dialog.answer.unwrap_or(false), dialog.checked)
    })
    .unwrap_or((false, false));
    // SAFETY: the owner is enabled before the dialog goes, so that it is
    // activated again rather than some other window.
    unsafe {
        if let Some(owner) = owner {
            let _ = EnableWindow(owner, true);
        }
        let _ = DestroyWindow(hwnd);
    }
    if let Some(code) = quit {
        // Leave the quit message for the main message loop.
        // SAFETY: plain call on this thread.
        unsafe { PostQuitMessage(code) };
    }
    answer
}

fn create(spec: &Spec) -> windows::core::Result<HWND> {
    // SAFETY: GetModuleHandleW(None) returns this executable's module.
    let instance = unsafe { GetModuleHandleW(None)? };
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_procedure),
        hInstance: instance.into(),
        // SAFETY: loads a system cursor.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    // SAFETY: the class structure is valid; registering twice fails harmlessly.
    unsafe { RegisterClassExW(&class) };

    // The dialog opens on the owner's display, or the one with the pointer.
    // SAFETY: plain queries with valid out parameters.
    let monitor = unsafe {
        match spec.owner {
            Some(owner) => MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
            None => {
                let mut cursor = POINT::default();
                let _ = GetCursorPos(&mut cursor);
                MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST)
            }
        }
    };
    let (mut dpi, mut dpi_y) = (96, 96);
    // SAFETY: as above.
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) };
    let graphics = Graphics::new()?;
    let text = Text {
        title: spec.title.to_owned(),
        content: spec.content.to_owned(),
        verification: spec.verification.map(str::to_owned),
        cancel: spec.cancel,
    };
    let scale = dpi as f32 / 96.0;
    let formats = Formats::new(&graphics, scale)?;
    let layout = compute_layout(&graphics, &formats, scale, &text);
    let (width, height) = window_size(&layout, dpi);

    // Centred on the owner, or on the display.
    let mut area = RECT::default();
    // SAFETY: plain queries with valid out parameters.
    unsafe {
        match spec.owner {
            Some(owner) => {
                let _ = GetWindowRect(owner, &mut area);
            }
            None => {
                let mut info = MONITORINFO {
                    cbSize: size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                let _ = GetMonitorInfoW(monitor, &mut info);
                area = info.rcWork;
            }
        }
    }
    let x = area.left + ((area.right - area.left) - width) / 2;
    let y = area.top + ((area.bottom - area.top) - height) / 2;
    let caption = HSTRING::from(spec.caption);
    // SAFETY: the class is registered and the strings outlive the call.
    let hwnd = unsafe {
        CreateWindowExW(
            DIALOG_EX_STYLE,
            CLASS_NAME,
            &caption,
            DIALOG_STYLE,
            x,
            y,
            width,
            height,
            spec.owner,
            None,
            Some(instance.into()),
            None,
        )?
    };
    let app_icon = (spec.icon == DialogIcon::App)
        .then(|| load_app_icon(dpi))
        .flatten();
    let mut dialog = Dialog {
        hwnd,
        owner: spec.owner,
        text,
        icon: spec.icon,
        checked: false,
        canvas: Some(Canvas::new(&graphics)?),
        graphics,
        palette: Palette::current(),
        formats,
        layout,
        app_icon,
        icons: WindowIcons::default(),
        hover: None,
        pressed: None,
        focus: Part::Ok,
        show_focus: false,
        tracking_mouse: false,
        answer: None,
    };
    paint::apply_window_appearance(hwnd, &dialog.palette);
    dialog.icons.set(hwnd, dpi);
    DIALOGS.with(|dialogs| dialogs.borrow_mut().push(dialog));
    Ok(hwnd)
}

fn load_app_icon(dpi: u32) -> Option<HICON> {
    let size = (ICON_SIZE * dpi as f32 / 96.0).round() as i32;
    // SAFETY: loads this executable's icon; destroyed with the dialog.
    unsafe {
        let module = GetModuleHandleW(None).ok()?;
        LoadIconWithScaleDown(Some(module.into()), APP_ICON_RESOURCE, size, size).ok()
    }
}

fn window_size(layout: &Layout, dpi: u32) -> (i32, i32) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: layout.client_width as i32,
        bottom: layout.footer.bottom as i32,
    };
    // SAFETY: `rect` is a valid RECT.
    let _ =
        unsafe { AdjustWindowRectExForDpi(&mut rect, DIALOG_STYLE, false, DIALOG_EX_STYLE, dpi) };
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn compute_layout(graphics: &Graphics, formats: &Formats, scale: f32, text: &Text) -> Layout {
    let px = |dips: f32| (dips * scale).round();
    let width = px(WIDTH);
    let icon = Bounds::new(px(PADDING), px(PADDING), px(ICON_SIZE), px(ICON_SIZE));
    let text_left = icon.right + px(ICON_GAP);
    let text_width = width - text_left - px(PADDING);
    let title_height = graphics
        .text_height(&text.title, &formats.title, text_width)
        .ceil();
    let content_height = graphics
        .text_height(&text.content, &formats.body, text_width)
        .ceil();
    let title = Bounds::new(text_left, px(PADDING), text_width, title_height);
    let content = Bounds::new(
        text_left,
        title.bottom + px(TITLE_GAP),
        text_width,
        content_height,
    );
    let mut bottom = content.bottom.max(icon.bottom);
    let (mut checkbox, mut checkbox_label) = (Bounds::default(), Bounds::default());
    if let Some(label) = &text.verification {
        let top = bottom + px(CHECKBOX_GAP);
        let available = text_width - px(CHECKBOX_SIZE) - px(CHECKBOX_LABEL_GAP);
        let label_width = graphics
            .text_width(label, &formats.label)
            .ceil()
            .min(available);
        let box_top = top + ((px(CONTROL_HEIGHT) - px(CHECKBOX_SIZE)) / 2.0).round();
        checkbox = Bounds::new(text_left, box_top, px(CHECKBOX_SIZE), px(CHECKBOX_SIZE));
        checkbox_label = Bounds::new(
            checkbox.right + px(CHECKBOX_LABEL_GAP),
            top,
            label_width,
            px(CONTROL_HEIGHT),
        );
        bottom = top + px(CONTROL_HEIGHT);
    }
    let footer = Bounds::new(0.0, bottom + px(PADDING), width, px(FOOTER_HEIGHT));
    // Two equal buttons fill the footer, as in Windows 11 content dialogs; a
    // lone OK takes the right-hand place.
    let button_width = ((width - 2.0 * px(PADDING) - px(BUTTON_GAP)) / 2.0).floor();
    let button_top = (footer.top + (footer.height() - px(CONTROL_HEIGHT)) / 2.0).round();
    let right = Bounds::new(
        width - px(PADDING) - button_width,
        button_top,
        button_width,
        px(CONTROL_HEIGHT),
    );
    let (ok, cancel) = if text.cancel {
        let left = Bounds::new(
            right.left - px(BUTTON_GAP) - button_width,
            button_top,
            button_width,
            px(CONTROL_HEIGHT),
        );
        (left, right)
    } else {
        (right, Bounds::default())
    };
    Layout {
        scale,
        client_width: width,
        icon,
        title,
        content,
        checkbox,
        checkbox_label,
        footer,
        ok,
        cancel,
    }
}

unsafe extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_GETOBJECT
        && let Some(result) = accessibility::handle_get_object(hwnd, wparam, lparam)
    {
        return result;
    }
    if message == accessibility::CALL_MESSAGE {
        accessibility::run_pending_call();
        return LRESULT(0);
    }
    let handled = with_dialog(hwnd, |dialog| {
        let focus_before = dialog.focus;
        let outcome = dialog.handle(message, wparam, lparam);
        let focus_changed = (dialog.focus != focus_before)
            .then(|| dialog.focus_index())
            .flatten();
        (outcome, focus_changed)
    });
    match handled {
        Some((Some((result, effects)), focus_changed)) => {
            for effect in effects {
                run_effect(hwnd, effect);
            }
            if let Some(index) = focus_changed {
                accessibility::focus_changed(hwnd, index);
            }
            result
        }
        _ => {
            if message == WM_DESTROY {
                accessibility::window_destroyed(hwnd);
                DIALOGS.with(|dialogs| {
                    if let Ok(mut dialogs) = dialogs.try_borrow_mut() {
                        dialogs.retain(|dialog| dialog.hwnd != hwnd);
                    }
                });
            }
            // SAFETY: default handling for every unhandled message.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

/// Work to do once the dialog's state is no longer borrowed.
enum Effect {
    Capture,
    ReleaseCapture,
    TrackMouseLeave,
    Resize { dpi: u32, suggested: RECT },
}

fn run_effect(hwnd: HWND, effect: Effect) {
    match effect {
        // SAFETY: capture calls on this thread's window.
        Effect::Capture => unsafe {
            SetCapture(hwnd);
        },
        // SAFETY: as above.
        Effect::ReleaseCapture => unsafe {
            let _ = ReleaseCapture();
        },
        Effect::TrackMouseLeave => {
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: `track` is a valid TRACKMOUSEEVENT.
            let _ = unsafe { TrackMouseEvent(&mut track) };
        }
        Effect::Resize { dpi, suggested } => {
            let size = with_dialog(hwnd, |dialog| {
                dialog.set_dpi(dpi);
                window_size(&dialog.layout, dpi)
            });
            if let Some((width, height)) = size {
                // SAFETY: repositions this thread's window.
                let _ = unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        suggested.left,
                        suggested.top,
                        width,
                        height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
            }
        }
    }
}

fn point_from_lparam(lparam: LPARAM) -> (f32, f32) {
    let x = (lparam.0 & 0xFFFF) as u16 as i16;
    let y = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16;
    (f32::from(x), f32::from(y))
}

fn key_is_down(key: VIRTUAL_KEY) -> bool {
    // SAFETY: GetKeyState has no preconditions.
    (unsafe { GetKeyState(i32::from(key.0)) }) < 0
}

impl Dialog {
    fn invalidate(&self) {
        // SAFETY: invalidates this thread's window.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    /// The parts of the dialog in reading order.
    fn parts(&self) -> Vec<Part> {
        let mut parts = vec![Part::Title, Part::Content];
        if self.text.verification.is_some() {
            parts.push(Part::Verification);
        }
        parts.push(Part::Ok);
        if self.text.cancel {
            parts.push(Part::Cancel);
        }
        parts
    }

    /// The parts that take the keyboard focus, in tab order.
    fn focus_order(&self) -> Vec<Part> {
        self.parts()
            .into_iter()
            .filter(|part| !matches!(part, Part::Title | Part::Content))
            .collect()
    }

    fn focus_index(&self) -> Option<usize> {
        self.parts().iter().position(|part| *part == self.focus)
    }

    fn part_bounds(&self, part: Part) -> Bounds {
        let layout = &self.layout;
        match part {
            Part::Title => layout.title,
            Part::Content => layout.content,
            // The check box with its label, padded like the Windows 11 control.
            Part::Verification => Bounds {
                left: layout.checkbox.left - (4.0 * layout.scale).round(),
                top: layout.checkbox_label.top,
                right: layout.checkbox_label.right + (6.0 * layout.scale).round(),
                bottom: layout.checkbox_label.bottom,
            },
            Part::Ok => layout.ok,
            Part::Cancel => layout.cancel,
        }
    }

    fn hit_test(&self, x: f32, y: f32) -> Option<Part> {
        self.focus_order()
            .into_iter()
            .find(|part| self.part_bounds(*part).contains(x, y))
    }

    fn set_dpi(&mut self, dpi: u32) {
        let scale = dpi as f32 / 96.0;
        if let Ok(formats) = Formats::new(&self.graphics, scale) {
            self.formats = formats;
        }
        self.layout = compute_layout(&self.graphics, &self.formats, scale, &self.text);
        if self.icon == DialogIcon::App {
            if let Some(icon) = self.app_icon.take() {
                // SAFETY: the icon was loaded by this dialog.
                let _ = unsafe { DestroyIcon(icon) };
            }
            self.app_icon = load_app_icon(dpi);
        }
        if let Some(canvas) = &mut self.canvas {
            canvas.reset();
        }
        self.icons.set(self.hwnd, dpi);
        self.invalidate();
    }

    fn activate(&mut self, part: Part) {
        match part {
            Part::Verification => self.checked = !self.checked,
            Part::Ok => self.answer = Some(true),
            Part::Cancel => self.answer = Some(false),
            Part::Title | Part::Content => {}
        }
        self.invalidate();
    }

    /// Escape and the close button answer Cancel, or OK when there is only OK.
    fn dismiss(&mut self) {
        self.answer = Some(!self.text.cancel);
    }

    fn handle(
        &mut self,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<(LRESULT, Vec<Effect>)> {
        let mut effects = Vec::new();
        let result = match message {
            WM_PAINT => {
                if let Some(mut canvas) = self.canvas.take() {
                    canvas.paint(self.hwnd, &self.graphics, |target| self.draw(target));
                    self.canvas = Some(canvas);
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_MOUSEMOVE => {
                let (x, y) = point_from_lparam(lparam);
                let hover = self.hit_test(x, y);
                if hover != self.hover {
                    self.hover = hover;
                    self.invalidate();
                }
                if !self.tracking_mouse {
                    self.tracking_mouse = true;
                    effects.push(Effect::TrackMouseLeave);
                }
                LRESULT(0)
            }
            WM_MOUSELEAVE => {
                self.tracking_mouse = false;
                if self.hover.take().is_some() {
                    self.invalidate();
                }
                LRESULT(0)
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                let (x, y) = point_from_lparam(lparam);
                if let Some(part) = self.hit_test(x, y) {
                    self.pressed = Some(part);
                    self.focus = part;
                    self.show_focus = false;
                    effects.push(Effect::Capture);
                    self.invalidate();
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let (x, y) = point_from_lparam(lparam);
                if let Some(pressed) = self.pressed.take() {
                    effects.push(Effect::ReleaseCapture);
                    if self.hit_test(x, y) == Some(pressed) {
                        self.activate(pressed);
                    }
                    self.invalidate();
                }
                LRESULT(0)
            }
            WM_CAPTURECHANGED => {
                if self.pressed.take().is_some() {
                    self.invalidate();
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let modified = key_is_down(VK_CONTROL) || key_is_down(VK_MENU);
                let key = VIRTUAL_KEY(wparam.0 as u16);
                if key == VK_TAB && !modified {
                    let order = self.focus_order();
                    let current = order
                        .iter()
                        .position(|part| *part == self.focus)
                        .unwrap_or(0);
                    let next = if key_is_down(VK_SHIFT) {
                        (current + order.len() - 1) % order.len()
                    } else {
                        (current + 1) % order.len()
                    };
                    self.focus = order[next];
                    self.show_focus = true;
                    self.invalidate();
                } else if key == VK_SPACE && !modified {
                    self.show_focus = true;
                    self.activate(self.focus);
                } else if key == VK_RETURN && !modified {
                    // Enter answers with the focused button, or with OK.
                    self.activate(if self.focus == Part::Cancel {
                        Part::Cancel
                    } else {
                        Part::Ok
                    });
                } else if key == VK_ESCAPE {
                    self.dismiss();
                } else {
                    return None;
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                self.dismiss();
                LRESULT(0)
            }
            accessibility::ACTIVATE_MESSAGE => {
                if let Some(part) = self.parts().get(wparam.0).copied() {
                    if self.focus_order().contains(&part) {
                        self.focus = part;
                    }
                    self.activate(part);
                }
                LRESULT(0)
            }
            WM_ACTIVATE => {
                self.invalidate();
                return None;
            }
            WM_SETTINGCHANGE
            | WM_THEMECHANGED
            | WM_SYSCOLORCHANGE
            | WM_DWMCOLORIZATIONCOLORCHANGED => {
                self.palette = Palette::current();
                paint::apply_window_appearance(self.hwnd, &self.palette);
                self.invalidate();
                return None;
            }
            WM_GETDPISCALEDSIZE => {
                let dpi = wparam.0 as u32;
                let scale = dpi as f32 / 96.0;
                let formats = Formats::new(&self.graphics, scale).ok()?;
                let layout = compute_layout(&self.graphics, &formats, scale, &self.text);
                let (width, height) = window_size(&layout, dpi);
                // SAFETY: lparam points to the SIZE to fill in.
                unsafe {
                    *(lparam.0 as *mut SIZE) = SIZE {
                        cx: width,
                        cy: height,
                    }
                };
                LRESULT(1)
            }
            WM_DPICHANGED => {
                // SAFETY: lparam points to the suggested window rectangle.
                let suggested = unsafe { *(lparam.0 as *const RECT) };
                effects.push(Effect::Resize {
                    dpi: (wparam.0 & 0xFFFF) as u32,
                    suggested,
                });
                LRESULT(0)
            }
            _ => return None,
        };
        Some((result, effects))
    }

    fn state_of(&self, part: Part) -> (bool, bool) {
        let pressed = self.pressed == Some(part) && self.hover == Some(part);
        let hovered = self.hover == Some(part) && (self.pressed.is_none() || pressed);
        (hovered, pressed)
    }

    fn draw(&self, target: &ID2D1RenderTarget) {
        let palette = &self.palette;
        let layout = &self.layout;
        let Some(painter) = Painter::new(target, &self.graphics, layout.scale) else {
            return;
        };
        painter.clear(if palette.backdrop {
            D2D1_COLOR_F::default()
        } else {
            palette.solid_background
        });
        // The message sits on a raised layer and the buttons on the backdrop
        // below it, as in Windows 11 content dialogs.
        let message = Bounds {
            left: 0.0,
            top: 0.0,
            right: layout.client_width,
            bottom: layout.footer.top,
        };
        painter.fill(message, 0.0, palette.card_fill);
        let divider = Bounds {
            top: layout.footer.top,
            bottom: layout.footer.top + painter.hairline(),
            ..message
        };
        painter.fill(divider, 0.0, palette.card_stroke);

        match self.icon {
            DialogIcon::App => {
                if let Some(icon) = self.app_icon
                    && let Ok(bitmap) = self.graphics.icon_bitmap(target, icon)
                {
                    let bounds = rect_f(
                        layout.icon.left,
                        layout.icon.top,
                        layout.icon.right,
                        layout.icon.bottom,
                    );
                    // SAFETY: drawing between BeginDraw and EndDraw.
                    unsafe {
                        target.DrawBitmap(
                            &bitmap,
                            Some(&bounds),
                            1.0,
                            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                            None,
                        )
                    };
                }
            }
            DialogIcon::Warning => painter.text(
                GLYPH_WARNING,
                &self.formats.glyph,
                layout.icon,
                palette.caution,
            ),
            DialogIcon::Information => {
                painter.text(GLYPH_INFO, &self.formats.glyph, layout.icon, palette.accent)
            }
        }
        painter.text(
            &self.text.title,
            &self.formats.title,
            layout.title,
            palette.text_primary,
        );
        painter.text(
            &self.text.content,
            &self.formats.body,
            layout.content,
            palette.text_primary,
        );

        if let Some(label) = &self.text.verification {
            let (hovered, pressed) = self.state_of(Part::Verification);
            painter.checkbox(
                layout.checkbox,
                &self.formats.check,
                palette,
                self.checked,
                hovered,
                pressed,
            );
            painter.text(
                label,
                &self.formats.label,
                layout.checkbox_label,
                palette.text_primary,
            );
        }
        let strings = strings();
        let (hovered, pressed) = self.state_of(Part::Ok);
        painter.button(
            layout.ok,
            strings.button_ok,
            &self.formats.button,
            palette,
            true,
            hovered,
            pressed,
        );
        if self.text.cancel {
            let (hovered, pressed) = self.state_of(Part::Cancel);
            painter.button(
                layout.cancel,
                strings.button_cancel,
                &self.formats.button,
                palette,
                false,
                hovered,
                pressed,
            );
        }
        if self.show_focus {
            painter.focus_ring(
                self.part_bounds(self.focus),
                painter.px(CONTROL_RADIUS),
                palette,
            );
        }
    }

    fn accessible_element(&self, index: usize) -> Option<AccessibleElement> {
        let part = *self.parts().get(index)?;
        let strings = strings();
        let bounds = self.part_bounds(part).to_rect();
        let mut top_left = POINT {
            x: bounds.left,
            y: bounds.top,
        };
        let mut bottom_right = POINT {
            x: bounds.right,
            y: bounds.bottom,
        };
        // SAFETY: converts coordinates for this thread's window.
        unsafe {
            let _ = ClientToScreen(self.hwnd, &mut top_left);
            let _ = ClientToScreen(self.hwnd, &mut bottom_right);
        }
        let (kind, name, toggled, id) = match part {
            Part::Title => (AccessibleKind::Text, self.text.title.clone(), None, "Title"),
            Part::Content => (
                AccessibleKind::Text,
                self.text.content.clone(),
                None,
                "Content",
            ),
            Part::Verification => (
                AccessibleKind::CheckBox,
                self.text.verification.clone().unwrap_or_default(),
                Some(self.checked),
                "DoNotShowAgain",
            ),
            Part::Ok => (
                AccessibleKind::Button,
                strings.button_ok.to_owned(),
                None,
                "OK",
            ),
            Part::Cancel => (
                AccessibleKind::Button,
                strings.button_cancel.to_owned(),
                None,
                "Cancel",
            ),
        };
        Some(AccessibleElement {
            kind,
            name,
            value: None,
            help: None,
            automation_id: id.to_owned(),
            screen_bounds: RECT {
                left: top_left.x,
                top: top_left.y,
                right: bottom_right.x,
                bottom: bottom_right.y,
            },
            focused: self.focus == part,
            toggled,
        })
    }
}

impl Drop for Dialog {
    fn drop(&mut self) {
        if let Some(icon) = self.app_icon.take() {
            // SAFETY: the icon was loaded by this dialog.
            let _ = unsafe { DestroyIcon(icon) };
        }
    }
}

// The interface UI Automation uses, by window.

pub fn accessible_count(hwnd: HWND) -> usize {
    with_dialog(hwnd, |dialog| dialog.parts().len()).unwrap_or(0)
}

pub fn accessible_element(hwnd: HWND, index: usize) -> Option<AccessibleElement> {
    with_dialog(hwnd, |dialog| dialog.accessible_element(index)).flatten()
}

pub fn accessible_hit_test(hwnd: HWND, x: f32, y: f32) -> Option<usize> {
    with_dialog(hwnd, |dialog| {
        let parts = dialog.parts();
        parts
            .iter()
            .position(|part| dialog.part_bounds(*part).contains(x, y))
    })
    .flatten()
}

pub fn accessible_focus(hwnd: HWND) -> Option<usize> {
    with_dialog(hwnd, |dialog| dialog.focus_index()).flatten()
}

pub fn accessible_set_focus(hwnd: HWND, index: usize) {
    with_dialog(hwnd, |dialog| {
        if let Some(part) = dialog.parts().get(index).copied()
            && dialog.focus_order().contains(&part)
        {
            dialog.focus = part;
            dialog.show_focus = true;
            dialog.invalidate();
        }
    });
}
