//! The Settings window: a recorder for every shortcut, the sign-in and run
//! mode options and a button to restore the default shortcuts.
//!
//! The window is drawn with Direct2D in the Windows 11 style, on a Mica
//! backdrop where available. All of its state lives in a thread-local and is
//! only borrowed while handling a message; anything that runs a modal loop
//! (dialogs, menus, mouse capture changes) runs after the borrow ends, as an
//! [`Effect`].

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::ID2D1RenderTarget;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteTextFormat,
};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, GetMonitorInfoW, HMONITOR, InvalidateRect, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromPoint, MonitorFromRect, MonitorFromWindow,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    TOOLTIPS_CLASSW, TTF_SUBCLASS, TTM_ADDTOOLW, TTM_DELTOOLW, TTM_SETMAXTIPWIDTH, TTS_ALWAYSTIP,
    TTS_NOPREFIX, TTTOOLINFOW, WM_MOUSELEAVE,
};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_F4, VK_LWIN, VK_MENU,
    VK_RETURN, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_HREDRAW, CS_VREDRAW, CheckMenuRadioItem, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, GetCursorPos, IDC_ARROW, IsIconic, LoadCursorW,
    MF_BYCOMMAND, MF_STRING, RegisterClassExW, SW_RESTORE, SW_SHOWNORMAL, SWP_NOACTIVATE,
    SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetWindowPos, ShowWindow, TPM_LEFTALIGN,
    TPM_RETURNCMD, TPM_TOPALIGN, TrackPopupMenuEx, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE,
    WM_CAPTURECHANGED, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_DWMCOLORIZATIONCOLORCHANGED,
    WM_ERASEBKGND, WM_GETDPISCALEDSIZE, WM_GETOBJECT, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETTINGCHANGE,
    WM_SYSCHAR, WM_SYSCOLORCHANGE, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_THEMECHANGED, WNDCLASSEXW,
    WS_CAPTION, WS_EX_TOPMOST, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU,
};
use windows::core::{HSTRING, PCWSTR, PWSTR, w};

use crate::accessibility;
use crate::action::WindowAction;
use crate::app::{self, BindingProblem, Snapshot};
use crate::dialogs::{self};
use crate::graphics::Graphics;
use crate::l10n::strings;
use crate::paint::{self, Bounds, CONTROL_RADIUS, Canvas, Painter, WindowIcons};
use crate::shortcut::{KeyBinding, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, modifier_label};
use crate::theme::Palette;

const CLASS_NAME: PCWSTR = w!("Spectacle.Settings");
const SETTINGS_WINDOW_STYLE: WINDOW_STYLE =
    WINDOW_STYLE(WS_OVERLAPPED.0 | WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0);

// Layout, in device independent pixels.
const PAGE_MARGIN: f32 = 20.0;
const PAGE_MARGIN_TOP: f32 = 12.0;
const COLUMN_GAP: f32 = 16.0;
const SECTION_GAP: f32 = 16.0;
const HEADER_HEIGHT: f32 = 20.0;
const HEADER_GAP: f32 = 6.0;
const ROW_HEIGHT: f32 = 40.0;
const CARD_PADDING: f32 = 16.0;
const LABEL_GAP: f32 = 16.0;
const RECORDER_HEIGHT: f32 = 30.0;
const RECORDER_INSET: f32 = 5.0;
const RECORDER_MINIMUM_WIDTH: f32 = 200.0;
const RECORDER_CONTENT_INSET: f32 = 8.0;
const BADGE_SIZE: f32 = 24.0;
const BADGE_INSET: f32 = 3.0;
const KEY_HEIGHT: f32 = 20.0;
const KEY_PADDING: f32 = 6.0;
const KEY_GAP: f32 = 4.0;
const FOOTER_GAP: f32 = 20.0;
const FOOTER_HEIGHT: f32 = 56.0;
const FOOTER_GROUP_GAP: f32 = 32.0;
const CONTROL_HEIGHT: f32 = 32.0;
const CHECKBOX_SIZE: f32 = 20.0;
const CHECKBOX_LABEL_GAP: f32 = 8.0;
const DROPDOWN_TEXT_INSET: f32 = 12.0;
const DROPDOWN_CHEVRON_WIDTH: f32 = 38.0;
const BUTTON_PADDING: f32 = 11.0;
const BUTTON_MINIMUM_WIDTH: f32 = 120.0;
const BODY_SIZE: f32 = 14.0;
const KEY_TEXT_SIZE: f32 = 12.0;

// Segoe Fluent Icons glyphs.
const GLYPH_CANCEL: &str = "\u{E711}";
const GLYPH_UNDO: &str = "\u{E7A7}";
const GLYPH_WARNING: &str = "\u{E7BA}";
const GLYPH_CHEVRON_DOWN: &str = "\u{E70D}";

const RUN_IN_SYSTEM_TRAY_COMMAND: u32 = 1;
const RUN_IN_BACKGROUND_COMMAND: u32 = 2;

thread_local! {
    static PREFERENCES: RefCell<Option<Preferences>> = const { RefCell::new(None) };
}

/// The sections of shortcuts, in two columns as in the macOS version.
fn columns() -> [Vec<(&'static str, Vec<WindowAction>)>; 2] {
    let strings = strings();
    [
        vec![
            (
                strings.section_basics,
                vec![WindowAction::Center, WindowAction::Fullscreen],
            ),
            (
                strings.section_halves,
                vec![
                    WindowAction::LeftHalf,
                    WindowAction::RightHalf,
                    WindowAction::TopHalf,
                    WindowAction::BottomHalf,
                ],
            ),
            (
                strings.section_corners,
                vec![
                    WindowAction::UpperLeft,
                    WindowAction::LowerLeft,
                    WindowAction::UpperRight,
                    WindowAction::LowerRight,
                ],
            ),
        ],
        vec![
            (
                strings.section_displays_and_thirds,
                vec![
                    WindowAction::NextDisplay,
                    WindowAction::PreviousDisplay,
                    WindowAction::NextThird,
                    WindowAction::PreviousThird,
                ],
            ),
            (
                strings.section_resize,
                vec![WindowAction::Larger, WindowAction::Smaller],
            ),
            (
                strings.section_history,
                vec![WindowAction::Undo, WindowAction::Redo],
            ),
        ],
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Element {
    Recorder(WindowAction),
    LaunchAtLogin,
    RunMode,
    RestoreDefaults,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Body,
    Badge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Hit {
    element: Element,
    part: Part,
}

struct Row {
    action: WindowAction,
    label: Bounds,
    recorder: Bounds,
}

struct Card {
    bounds: Bounds,
    title: &'static str,
    header: Bounds,
    rows: Vec<Row>,
}

#[derive(Default)]
struct Layout {
    scale: f32,
    client_width: f32,
    client_height: f32,
    cards: Vec<Card>,
    footer: Bounds,
    checkbox: Bounds,
    checkbox_label: Bounds,
    run_label: Bounds,
    run_mode: Bounds,
    restore_defaults: Bounds,
}

impl Layout {
    fn px(&self, dips: f32) -> f32 {
        (dips * self.scale).round()
    }

    /// The width of a hairline: one device independent pixel, at least one
    /// physical pixel.
    fn hairline(&self) -> f32 {
        self.px(1.0).max(1.0)
    }

    fn badge(&self, recorder: Bounds) -> Bounds {
        let size = self.px(BADGE_SIZE);
        let left = recorder.right - self.px(BADGE_INSET) - size;
        let top = (recorder.top + (recorder.height() - size) / 2.0).round();
        Bounds::new(left, top, size, size)
    }

    fn warning(&self, recorder: Bounds) -> Bounds {
        let badge = self.badge(recorder);
        Bounds::new(
            badge.left - badge.width(),
            badge.top,
            badge.width(),
            badge.height(),
        )
    }

    /// The check box with its label, padded like the Windows 11 control.
    fn checkbox_hit(&self) -> Bounds {
        Bounds {
            left: self.checkbox.left - self.px(4.0),
            top: self.run_mode.top,
            right: self.checkbox_label.right + self.px(6.0),
            bottom: self.run_mode.bottom,
        }
    }

    fn element_bounds(&self, element: Element) -> Bounds {
        match element {
            Element::Recorder(action) => self
                .cards
                .iter()
                .flat_map(|card| card.rows.iter())
                .find(|row| row.action == action)
                .map(|row| row.recorder)
                .unwrap_or_default(),
            Element::LaunchAtLogin => self.checkbox_hit(),
            Element::RunMode => self.run_mode,
            Element::RestoreDefaults => self.restore_defaults,
        }
    }

    fn hit_test(&self, x: f32, y: f32) -> Option<Hit> {
        for row in self.cards.iter().flat_map(|card| card.rows.iter()) {
            if row.recorder.contains(x, y) {
                let element = Element::Recorder(row.action);
                let part = if self.badge(row.recorder).contains(x, y) {
                    Part::Badge
                } else {
                    Part::Body
                };
                return Some(Hit { element, part });
            }
        }
        [
            Element::LaunchAtLogin,
            Element::RunMode,
            Element::RestoreDefaults,
        ]
        .into_iter()
        .find(|element| self.element_bounds(*element).contains(x, y))
        .map(|element| Hit {
            element,
            part: Part::Body,
        })
    }

    /// Focusable elements in tab order.
    fn focus_order(&self) -> Vec<Element> {
        let mut order: Vec<Element> = self
            .cards
            .iter()
            .flat_map(|card| card.rows.iter())
            .map(|row| Element::Recorder(row.action))
            .collect();
        order.extend([
            Element::LaunchAtLogin,
            Element::RunMode,
            Element::RestoreDefaults,
        ]);
        order
    }
}

struct TextFormats {
    body: IDWriteTextFormat,
    body_centered: IDWriteTextFormat,
    body_strong: IDWriteTextFormat,
    key: IDWriteTextFormat,
    icon_small: IDWriteTextFormat,
    icon: IDWriteTextFormat,
    icon_large: IDWriteTextFormat,
}

impl TextFormats {
    fn new(graphics: &Graphics, scale: f32) -> windows::core::Result<TextFormats> {
        let size = |dips: f32| dips * scale;
        Ok(TextFormats {
            body: graphics.text_format(
                size(BODY_SIZE),
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?,
            body_centered: graphics.text_format(
                size(BODY_SIZE),
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?,
            body_strong: graphics.text_format(
                size(BODY_SIZE),
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?,
            key: graphics.text_format(
                size(KEY_TEXT_SIZE),
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?,
            icon_small: graphics.icon_format(size(10.0))?,
            icon: graphics.icon_format(size(12.0))?,
            icon_large: graphics.icon_format(size(14.0))?,
        })
    }
}

/// Work to do once the window's state is no longer borrowed.
enum Effect {
    Capture,
    ReleaseCapture,
    TrackMouseLeave,
    Recorded(WindowAction, KeyBinding),
    Clear(WindowAction),
    ToggleLaunchAtLogin,
    ShowRunModeMenu,
    RestoreDefaults,
    Resize { dpi: u32, suggested: RECT },
    Close,
    FocusChanged(usize),
}

/// What assistive technologies are told about a control.
pub enum AccessibleKind {
    Recorder,
    CheckBox,
    RunMode,
    Button,
    /// Text that is read, such as a dialog's message.
    Text,
}

pub struct AccessibleElement {
    pub kind: AccessibleKind,
    pub name: String,
    pub value: Option<String>,
    pub help: Option<String>,
    pub automation_id: String,
    pub screen_bounds: RECT,
    pub focused: bool,
    pub toggled: Option<bool>,
}

/// The number of controls, in tab order.
pub fn accessible_count() -> usize {
    with_preferences(|p| p.layout.focus_order().len()).unwrap_or(0)
}

pub fn accessible_element(index: usize) -> Option<AccessibleElement> {
    with_preferences(|p| p.accessible_element(index)).flatten()
}

/// The control at a point in client coordinates.
pub fn accessible_hit_test(x: f32, y: f32) -> Option<usize> {
    with_preferences(|p| {
        let hit = p.layout.hit_test(x, y)?;
        p.layout
            .focus_order()
            .iter()
            .position(|element| *element == hit.element)
    })
    .flatten()
}

pub fn accessible_focus() -> Option<usize> {
    with_preferences(|p| p.focus_index()).flatten()
}

pub fn accessible_set_focus(index: usize) {
    with_preferences(|p| {
        if let Some(element) = p.layout.focus_order().get(index).copied() {
            p.focus = Some(element);
            p.show_focus = true;
            p.invalidate();
        }
    });
}

struct Preferences {
    hwnd: HWND,
    graphics: Graphics,
    canvas: Option<Canvas>,
    palette: Palette,
    dpi: u32,
    formats: TextFormats,
    layout: Layout,
    snapshot: Snapshot,
    hover: Option<Hit>,
    pressed: Option<Hit>,
    focus: Option<Element>,
    show_focus: bool,
    active: bool,
    tracking_mouse: bool,
    recording: Option<WindowAction>,
    held_modifiers: u32,
    tooltip: Option<HWND>,
    tooltip_texts: Vec<HSTRING>,
    icons: WindowIcons,
}

/// Shows the Settings window, creating it if needed.
pub fn show() {
    if let Some(hwnd) = hwnd() {
        // A dialog of the Settings window comes forward in its place.
        if dialogs::activate_dialog_of(hwnd) {
            return;
        }
        // SAFETY: the window belongs to this thread.
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
        return;
    }
    if let Err(error) = create() {
        app::report_error("Spectacle could not open its settings.", &error.message());
    }
}

pub fn hwnd() -> Option<HWND> {
    PREFERENCES.with(|preferences| preferences.try_borrow().ok()?.as_ref().map(|p| p.hwnd))
}

pub fn close() {
    if let Some(hwnd) = hwnd() {
        // SAFETY: the window belongs to this thread.
        let _ = unsafe { DestroyWindow(hwnd) };
    }
}

/// Re-reads the shortcuts and options shown.
pub fn refresh() {
    let Some(snapshot) = app::snapshot() else {
        return;
    };
    with_preferences(|preferences| {
        preferences.snapshot = snapshot;
        preferences.update_tooltips();
        preferences.invalidate();
    });
}

fn with_preferences<R>(f: impl FnOnce(&mut Preferences) -> R) -> Option<R> {
    PREFERENCES.with(|preferences| preferences.try_borrow_mut().ok()?.as_mut().map(f))
}

fn create() -> windows::core::Result<()> {
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

    let mut cursor = POINT::default();
    // SAFETY: plain queries with valid out parameters.
    let monitor = unsafe {
        let _ = GetCursorPos(&mut cursor);
        MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST)
    };
    let (mut dpi_x, mut dpi_y) = (96, 96);
    // SAFETY: as above.
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let snapshot = app::snapshot().unwrap_or_default();
    let graphics = Graphics::new()?;
    let work = work_area(monitor).unwrap_or_default();
    let (formats, layout) = fitted_layout(&graphics, dpi_x, Some(work))?;
    let (width, height) = window_size(&layout, dpi_x);
    let x = work.left + ((work.right - work.left) - width).max(0) / 2;
    let y = work.top + ((work.bottom - work.top) - height).max(0) / 2;
    let title = HSTRING::from(format!("Spectacle {}", crate::VERSION));
    // SAFETY: the class is registered and the strings outlive the call.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS_NAME,
            &title,
            SETTINGS_WINDOW_STYLE,
            x,
            y,
            width,
            height,
            None,
            None,
            Some(instance.into()),
            None,
        )?
    };
    let canvas = Canvas::new(&graphics)?;
    let mut preferences = Preferences {
        hwnd,
        graphics,
        canvas: Some(canvas),
        palette: Palette::current(),
        dpi: dpi_x,
        formats,
        layout,
        snapshot,
        hover: None,
        pressed: None,
        focus: None,
        show_focus: false,
        active: true,
        tracking_mouse: false,
        recording: None,
        held_modifiers: 0,
        tooltip: None,
        tooltip_texts: Vec::new(),
        icons: WindowIcons::default(),
    };
    // The window may have been created on a display other than the one the
    // layout was computed for.
    // SAFETY: the window was just created on this thread.
    let actual_dpi = unsafe { GetDpiForWindow(hwnd) };
    if actual_dpi != 0 && actual_dpi != preferences.dpi {
        // SAFETY: as above.
        preferences.set_dpi(
            actual_dpi,
            work_area(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }),
        );
        let (width, height) = window_size(&preferences.layout, actual_dpi);
        // SAFETY: as above.
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE,
            )
        };
    }
    preferences.apply_appearance();
    preferences.set_icons();
    preferences.create_tooltip();
    preferences.update_tooltips();
    PREFERENCES.with(|cell| *cell.borrow_mut() = Some(preferences));
    // SAFETY: the window belongs to this thread.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

fn work_area(monitor: HMONITOR) -> Option<RECT> {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a properly sized MONITORINFO.
    unsafe { GetMonitorInfoW(monitor, &mut info) }
        .as_bool()
        .then_some(info.rcWork)
}

/// Text formats and layout for a display's scale, shrunk where needed so the
/// whole window fits within the display's work area.
fn fitted_layout(
    graphics: &Graphics,
    dpi: u32,
    work_area: Option<RECT>,
) -> windows::core::Result<(TextFormats, Layout)> {
    let mut scale = dpi as f32 / 96.0;
    let mut formats = TextFormats::new(graphics, scale)?;
    let mut layout = compute_layout(graphics, &formats, scale);
    let Some(work) = work_area else {
        return Ok((formats, layout));
    };
    let (available_width, available_height) = (
        (work.right - work.left) as f32,
        (work.bottom - work.top) as f32,
    );
    // The title bar does not shrink with the content, so a few passes may be
    // needed to converge.
    for _ in 0..4 {
        let (width, height) = window_size(&layout, dpi);
        let ratio = (available_width / width as f32).min(available_height / height as f32);
        if ratio >= 1.0 || available_width <= 0.0 || available_height <= 0.0 {
            break;
        }
        scale *= ratio;
        formats = TextFormats::new(graphics, scale)?;
        layout = compute_layout(graphics, &formats, scale);
    }
    Ok((formats, layout))
}

fn window_size(layout: &Layout, dpi: u32) -> (i32, i32) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: layout.client_width as i32,
        bottom: layout.client_height as i32,
    };
    // SAFETY: `rect` is a valid RECT.
    let _ = unsafe {
        AdjustWindowRectExForDpi(
            &mut rect,
            SETTINGS_WINDOW_STYLE,
            false,
            WINDOW_EX_STYLE::default(),
            dpi,
        )
    };
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn compute_layout(graphics: &Graphics, formats: &TextFormats, scale: f32) -> Layout {
    let strings = strings();
    let mut layout = Layout {
        scale,
        ..Default::default()
    };
    let px = |dips: f32| (dips * scale).round();
    let measure = |text: &str, format: &IDWriteTextFormat| graphics.text_width(text, format).ceil();

    let label_width = WindowAction::ALL
        .iter()
        .map(|action| measure(strings.action(*action), &formats.body))
        .fold(0.0, f32::max);
    // Wide enough for every modifier and a short key, or the placeholders.
    let keys_width = [strings.shift_key, "Win", "Ctrl", "Alt", "PgDn"]
        .iter()
        .map(|key| measure(key, &formats.key) + 2.0 * px(KEY_PADDING))
        .sum::<f32>()
        + 4.0 * px(KEY_GAP);
    let placeholder_width = [
        strings.recorder_click_to_record,
        strings.recorder_enter_shortcut,
    ]
    .iter()
    .map(|text| measure(text, &formats.body))
    .fold(0.0, f32::max);
    let recorder_width = (keys_width.max(placeholder_width)
        + px(RECORDER_CONTENT_INSET) * 2.0
        + px(BADGE_SIZE)
        + px(BADGE_INSET))
    .max(px(RECORDER_MINIMUM_WIDTH));
    let mut column_width =
        px(CARD_PADDING) + label_width + px(LABEL_GAP) + recorder_width + px(RECORDER_INSET);

    let launch_width = measure(strings.launch_at_login, &formats.body);
    let run_label_width = measure(strings.run_mode_label, &formats.body);
    let run_mode_width = [strings.run_in_system_tray, strings.run_in_background]
        .iter()
        .map(|text| measure(text, &formats.body))
        .fold(0.0, f32::max)
        + px(DROPDOWN_TEXT_INSET)
        + px(DROPDOWN_CHEVRON_WIDTH);
    let button_width = (measure(strings.restore_defaults, &formats.body)
        + 2.0 * px(BUTTON_PADDING))
    .max(px(BUTTON_MINIMUM_WIDTH));
    let footer_content_width = px(CHECKBOX_SIZE)
        + px(CHECKBOX_LABEL_GAP)
        + launch_width
        + px(FOOTER_GROUP_GAP)
        + run_label_width
        + px(CHECKBOX_LABEL_GAP)
        + run_mode_width
        + px(FOOTER_GROUP_GAP)
        + button_width
        + 2.0 * px(CARD_PADDING);
    let content_width = (2.0 * column_width + px(COLUMN_GAP)).max(footer_content_width);
    column_width = ((content_width - px(COLUMN_GAP)) / 2.0).floor();
    let content_width = 2.0 * column_width + px(COLUMN_GAP);

    let mut bottom: f32 = 0.0;
    for (index, sections) in columns().into_iter().enumerate() {
        let left = px(PAGE_MARGIN) + index as f32 * (column_width + px(COLUMN_GAP));
        let mut top = px(PAGE_MARGIN_TOP);
        for (title, actions) in sections {
            let header = Bounds::new(left, top, column_width, px(HEADER_HEIGHT));
            top = header.bottom + px(HEADER_GAP);
            let bounds = Bounds::new(
                left,
                top,
                column_width,
                px(ROW_HEIGHT) * actions.len() as f32,
            );
            let rows = actions
                .into_iter()
                .enumerate()
                .map(|(i, action)| {
                    let row_top = top + px(ROW_HEIGHT) * i as f32;
                    let recorder_top =
                        row_top + ((px(ROW_HEIGHT) - px(RECORDER_HEIGHT)) / 2.0).round();
                    let recorder = Bounds::new(
                        bounds.right - px(RECORDER_INSET) - recorder_width,
                        recorder_top,
                        recorder_width,
                        px(RECORDER_HEIGHT),
                    );
                    let label = Bounds {
                        left: left + px(CARD_PADDING),
                        top: row_top,
                        right: recorder.left - px(LABEL_GAP),
                        bottom: row_top + px(ROW_HEIGHT),
                    };
                    Row {
                        action,
                        label,
                        recorder,
                    }
                })
                .collect();
            top = bounds.bottom;
            layout.cards.push(Card {
                bounds,
                title,
                header,
                rows,
            });
            bottom = bottom.max(top);
            top += px(SECTION_GAP);
        }
    }

    let footer = Bounds::new(
        px(PAGE_MARGIN),
        bottom + px(FOOTER_GAP),
        content_width,
        px(FOOTER_HEIGHT),
    );
    let control_top = (footer.top + (footer.height() - px(CONTROL_HEIGHT)) / 2.0).round();
    let checkbox_top = (footer.top + (footer.height() - px(CHECKBOX_SIZE)) / 2.0).round();
    layout.checkbox = Bounds::new(
        footer.left + px(CARD_PADDING),
        checkbox_top,
        px(CHECKBOX_SIZE),
        px(CHECKBOX_SIZE),
    );
    layout.checkbox_label = Bounds::new(
        layout.checkbox.right + px(CHECKBOX_LABEL_GAP),
        control_top,
        launch_width,
        px(CONTROL_HEIGHT),
    );
    layout.run_label = Bounds::new(
        layout.checkbox_label.right + px(FOOTER_GROUP_GAP),
        control_top,
        run_label_width,
        px(CONTROL_HEIGHT),
    );
    layout.run_mode = Bounds::new(
        layout.run_label.right + px(CHECKBOX_LABEL_GAP),
        control_top,
        run_mode_width,
        px(CONTROL_HEIGHT),
    );
    layout.restore_defaults = Bounds::new(
        footer.right - px(CARD_PADDING) - button_width,
        control_top,
        button_width,
        px(CONTROL_HEIGHT),
    );
    layout.footer = footer;
    layout.client_width = content_width + 2.0 * px(PAGE_MARGIN);
    layout.client_height = footer.bottom + px(PAGE_MARGIN);
    layout
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
    let handled = with_preferences(|preferences| {
        let focus_before = preferences.focus;
        let mut outcome = preferences.handle(message, wparam, lparam);
        if preferences.focus != focus_before
            && let (Some((_, effects)), Some(index)) = (&mut outcome, preferences.focus_index())
        {
            effects.push(Effect::FocusChanged(index));
        }
        outcome
    });
    match handled {
        Some(Some((result, effects))) => {
            for effect in effects {
                run_effect(hwnd, effect);
            }
            result
        }
        _ => {
            if message == WM_DESTROY {
                accessibility::window_destroyed(hwnd);
                PREFERENCES.with(|cell| {
                    if let Ok(mut preferences) = cell.try_borrow_mut() {
                        *preferences = None;
                    }
                });
            }
            // SAFETY: default handling for every unhandled message.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
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
        Effect::Recorded(action, binding) => match app::validate_binding(action, binding) {
            Some(Ok(true)) => {
                app::set_shortcut(action, Some(binding));
                refresh();
            }
            Some(Ok(false)) | None => {}
            Some(Err(problem)) => {
                let shortcut = binding.display_string(strings().shift_key);
                dialogs::shortcut_in_use(
                    hwnd,
                    &shortcut,
                    problem == BindingProblem::UsedByAnotherApplication,
                );
            }
        },
        Effect::Clear(action) => {
            app::set_shortcut(action, None);
            refresh();
        }
        Effect::ToggleLaunchAtLogin => {
            let enabled = with_preferences(|p| p.snapshot.launch_at_login).unwrap_or(false);
            app::set_launch_at_login(!enabled);
            refresh();
        }
        Effect::ShowRunModeMenu => show_run_mode_menu(hwnd),
        Effect::RestoreDefaults => {
            if dialogs::confirm_restore_defaults(hwnd) {
                app::restore_default_shortcuts();
                refresh();
            }
        }
        Effect::Resize { dpi, suggested } => {
            // SAFETY: plain monitor query.
            let monitor = unsafe { MonitorFromRect(&suggested, MONITOR_DEFAULTTONEAREST) };
            let size = with_preferences(|p| {
                p.set_dpi(dpi, work_area(monitor));
                window_size(&p.layout, dpi)
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
        Effect::Close => {
            // SAFETY: destroys this thread's window.
            let _ = unsafe { DestroyWindow(hwnd) };
        }
        Effect::FocusChanged(index) => accessibility::focus_changed(hwnd, index),
    }
}

fn show_run_mode_menu(hwnd: HWND) {
    let Some((bounds, in_tray)) =
        with_preferences(|p| (p.layout.run_mode, p.snapshot.run_in_system_tray))
    else {
        return;
    };
    let strings = strings();
    let tray_text = HSTRING::from(strings.run_in_system_tray);
    let background_text = HSTRING::from(strings.run_in_background);
    let mut origin = POINT {
        x: bounds.left as i32,
        y: bounds.bottom as i32,
    };
    // SAFETY: menu calls on this thread with strings that outlive them.
    let command = unsafe {
        let _ = ClientToScreen(hwnd, &mut origin);
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            RUN_IN_SYSTEM_TRAY_COMMAND as usize,
            &tray_text,
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            RUN_IN_BACKGROUND_COMMAND as usize,
            &background_text,
        );
        let selected = if in_tray {
            RUN_IN_SYSTEM_TRAY_COMMAND
        } else {
            RUN_IN_BACKGROUND_COMMAND
        };
        let _ = CheckMenuRadioItem(
            menu,
            RUN_IN_SYSTEM_TRAY_COMMAND,
            RUN_IN_BACKGROUND_COMMAND,
            selected,
            MF_BYCOMMAND.0,
        );
        let command = TrackPopupMenuEx(
            menu,
            (TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN).0,
            origin.x,
            origin.y,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        command.0 as u32
    };
    match command {
        RUN_IN_SYSTEM_TRAY_COMMAND if !in_tray => app::set_run_in_system_tray(true),
        RUN_IN_BACKGROUND_COMMAND if in_tray => {
            let suppressed = app::snapshot().is_some_and(|s| s.background_alert_suppressed);
            if suppressed {
                app::set_run_in_system_tray(false);
            } else {
                let (confirmed, suppress) = dialogs::confirm_running_in_background(hwnd);
                app::set_background_alert_suppressed(suppress);
                if confirmed {
                    app::set_run_in_system_tray(false);
                }
            }
        }
        _ => {}
    }
    refresh();
}

/// Unsigned 16-bit coordinates packed in an LPARAM, as signed values.
fn point_from_lparam(lparam: LPARAM) -> (f32, f32) {
    let x = (lparam.0 & 0xFFFF) as u16 as i16;
    let y = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16;
    (f32::from(x), f32::from(y))
}

/// The Segoe Fluent Icons arrow for an arrow key's label.
fn arrow_glyph(label: &str) -> Option<&'static str> {
    match label {
        "\u{2190}" => Some("\u{E72B}"),
        "\u{2192}" => Some("\u{E72A}"),
        "\u{2191}" => Some("\u{E74A}"),
        "\u{2193}" => Some("\u{E74B}"),
        _ => None,
    }
}

fn key_is_down(key: VIRTUAL_KEY) -> bool {
    // SAFETY: GetKeyState has no preconditions.
    (unsafe { GetKeyState(i32::from(key.0)) }) < 0
}

fn held_modifiers() -> u32 {
    let mut modifiers = 0;
    if key_is_down(VK_LWIN) || key_is_down(VK_RWIN) {
        modifiers |= MOD_WIN;
    }
    if key_is_down(VK_CONTROL) {
        modifiers |= MOD_CONTROL;
    }
    if key_is_down(VK_MENU) {
        modifiers |= MOD_ALT;
    }
    if key_is_down(VK_SHIFT) {
        modifiers |= MOD_SHIFT;
    }
    modifiers
}

fn is_modifier_key(key: u16) -> bool {
    matches!(key, 0x10..=0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

impl Preferences {
    fn focus_index(&self) -> Option<usize> {
        let focus = self.focus?;
        self.layout
            .focus_order()
            .iter()
            .position(|element| *element == focus)
    }

    fn accessible_element(&self, index: usize) -> Option<AccessibleElement> {
        let element = *self.layout.focus_order().get(index)?;
        let strings = strings();
        let bounds = self.layout.element_bounds(element).to_rect();
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
        let screen_bounds = RECT {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        };
        let focused = self.active && self.focus == Some(element);
        let accessible =
            |kind, name: &str, value: Option<String>, help: Option<String>, id: &str, toggled| {
                AccessibleElement {
                    kind,
                    name: name.to_owned(),
                    value,
                    help,
                    automation_id: id.to_owned(),
                    screen_bounds,
                    focused,
                    toggled,
                }
            };
        Some(match element {
            Element::Recorder(action) => {
                let binding = self.snapshot.binding(action);
                let value = if self.recording == Some(action) {
                    strings.recorder_enter_shortcut.to_owned()
                } else {
                    binding.map_or_else(
                        || strings.recorder_click_to_record.to_owned(),
                        |b| b.spoken_string(strings.shift_key),
                    )
                };
                let help = (binding.is_some() && self.snapshot.is_unavailable(action))
                    .then(|| strings.recorder_not_registered.to_owned());
                accessible(
                    AccessibleKind::Recorder,
                    strings.action(action),
                    Some(value),
                    help,
                    action.shortcut_name(),
                    None,
                )
            }
            Element::LaunchAtLogin => accessible(
                AccessibleKind::CheckBox,
                strings.launch_at_login,
                None,
                None,
                "LaunchAtSignIn",
                Some(self.snapshot.launch_at_login),
            ),
            Element::RunMode => {
                let mode = if self.snapshot.run_in_system_tray {
                    strings.run_in_system_tray
                } else {
                    strings.run_in_background
                };
                accessible(
                    AccessibleKind::RunMode,
                    strings.run_mode_label,
                    Some(mode.to_owned()),
                    None,
                    "RunMode",
                    None,
                )
            }
            Element::RestoreDefaults => accessible(
                AccessibleKind::Button,
                strings.restore_defaults.trim_end_matches('\u{2026}'),
                None,
                None,
                "RestoreDefaults",
                None,
            ),
        })
    }

    fn invalidate(&self) {
        // SAFETY: invalidates this thread's window.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    fn set_dpi(&mut self, dpi: u32, work_area: Option<RECT>) {
        self.dpi = dpi;
        if let Ok((formats, layout)) = fitted_layout(&self.graphics, dpi, work_area) {
            self.formats = formats;
            self.layout = layout;
        }
        if let Some(canvas) = &mut self.canvas {
            canvas.reset();
        }
        self.set_icons();
        self.update_tooltips();
        self.invalidate();
    }

    fn apply_appearance(&mut self) {
        self.palette = Palette::current();
        paint::apply_window_appearance(self.hwnd, &self.palette);
        self.invalidate();
    }

    fn set_icons(&mut self) {
        self.icons.set(self.hwnd, self.dpi);
    }

    fn create_tooltip(&mut self) {
        // SAFETY: creates a tooltip window owned by this window.
        let tooltip = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASSW,
                None,
                WINDOW_STYLE(WS_POPUP.0 | TTS_ALWAYSTIP | TTS_NOPREFIX),
                0,
                0,
                0,
                0,
                Some(self.hwnd),
                None,
                GetModuleHandleW(None).ok().map(Into::into),
                None,
            )
        };
        if let Ok(tooltip) = tooltip {
            // SAFETY: configures the tooltip just created.
            unsafe { SendMessageW(tooltip, TTM_SETMAXTIPWIDTH, None, Some(LPARAM(400))) };
            self.tooltip = Some(tooltip);
        }
    }

    /// Describes the recorder buttons and warnings to the tooltip control.
    fn update_tooltips(&mut self) {
        let Some(tooltip) = self.tooltip else {
            return;
        };
        let strings = strings();
        let mut tool = TTTOOLINFOW {
            cbSize: size_of::<TTTOOLINFOW>() as u32,
            uFlags: TTF_SUBCLASS,
            hwnd: self.hwnd,
            ..Default::default()
        };
        for id in 0..(WindowAction::ALL.len() * 2) {
            tool.uId = id + 1;
            // SAFETY: removes a tool of this window's tooltip, if present.
            unsafe {
                SendMessageW(
                    tooltip,
                    TTM_DELTOOLW,
                    None,
                    Some(LPARAM(&tool as *const _ as isize)),
                )
            };
        }
        self.tooltip_texts.clear();
        let rows: Vec<(WindowAction, Bounds)> = self
            .layout
            .cards
            .iter()
            .flat_map(|card| card.rows.iter())
            .map(|row| (row.action, row.recorder))
            .collect();
        for (action, recorder) in rows {
            let recording = self.recording == Some(action);
            let binding = self.snapshot.binding(action);
            let badge_text = match (recording, binding.is_some()) {
                (true, true) => Some(strings.recorder_use_existing),
                (true, false) => Some(strings.recorder_stop_recording),
                (false, true) => Some(strings.recorder_clear_shortcut),
                (false, false) => None,
            };
            let mut tools = Vec::new();
            if let Some(text) = badge_text {
                tools.push((action.index() * 2 + 1, self.layout.badge(recorder), text));
            }
            if !recording && binding.is_some() && self.snapshot.is_unavailable(action) {
                tools.push((
                    action.index() * 2 + 2,
                    self.layout.warning(recorder),
                    strings.recorder_not_registered,
                ));
            }
            for (id, bounds, text) in tools {
                let text = HSTRING::from(text);
                tool.uId = id;
                tool.rect = bounds.to_rect();
                tool.lpszText = PWSTR(text.as_ptr() as *mut u16);
                // SAFETY: the tooltip copies the text; `tool` is valid.
                unsafe {
                    SendMessageW(
                        tooltip,
                        TTM_ADDTOOLW,
                        None,
                        Some(LPARAM(&tool as *const _ as isize)),
                    )
                };
                self.tooltip_texts.push(text);
            }
        }
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
                self.paint();
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_MOUSEMOVE => {
                let (x, y) = point_from_lparam(lparam);
                let hover = self.layout.hit_test(x, y);
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
                let hit = self.layout.hit_test(x, y);
                if let Some(recording) = self.recording
                    && hit.map(|hit| hit.element) != Some(Element::Recorder(recording))
                {
                    self.stop_recording();
                }
                self.show_focus = false;
                if let Some(hit) = hit {
                    self.focus = Some(hit.element);
                    self.pressed = Some(hit);
                    effects.push(Effect::Capture);
                }
                self.invalidate();
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let (x, y) = point_from_lparam(lparam);
                let hit = self.layout.hit_test(x, y);
                if let Some(pressed) = self.pressed.take() {
                    effects.push(Effect::ReleaseCapture);
                    if hit == Some(pressed) {
                        self.activate(pressed, &mut effects);
                    }
                }
                self.invalidate();
                LRESULT(0)
            }
            WM_CAPTURECHANGED => {
                if self.pressed.take().is_some() {
                    self.invalidate();
                }
                LRESULT(0)
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let key = wparam.0 as u16;
                if self.recording.is_some() {
                    self.record_key(key, &mut effects);
                    LRESULT(0)
                } else if self.handle_navigation_key(key, &mut effects) {
                    LRESULT(0)
                } else {
                    return None;
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                if self.recording.is_some() {
                    let modifiers = held_modifiers();
                    if modifiers != self.held_modifiers {
                        self.held_modifiers = modifiers;
                        self.invalidate();
                    }
                    // Swallow Alt and F10 so they do not open the window menu.
                    LRESULT(0)
                } else {
                    return None;
                }
            }
            WM_SYSCHAR if self.recording.is_some() => LRESULT(0),
            WM_ACTIVATE => {
                self.active = (wparam.0 & 0xFFFF) != 0;
                if !self.active {
                    self.stop_recording();
                    self.pressed = None;
                } else if let Some(snapshot) = app::snapshot() {
                    // Task Manager may have changed the sign-in setting.
                    self.snapshot = snapshot;
                    self.update_tooltips();
                }
                self.invalidate();
                return None;
            }
            WM_KILLFOCUS => {
                self.stop_recording();
                self.invalidate();
                return None;
            }
            WM_SETTINGCHANGE
            | WM_THEMECHANGED
            | WM_SYSCOLORCHANGE
            | WM_DWMCOLORIZATIONCOLORCHANGED => {
                self.apply_appearance();
                return None;
            }
            WM_GETDPISCALEDSIZE => {
                // The window is being dragged onto a display with another
                // scale; that display is the one under the pointer.
                let dpi = wparam.0 as u32;
                let mut cursor = POINT::default();
                // SAFETY: plain queries with valid out parameters.
                let monitor = unsafe {
                    let _ = GetCursorPos(&mut cursor);
                    MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST)
                };
                let (_, layout) = fitted_layout(&self.graphics, dpi, work_area(monitor)).ok()?;
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
            WM_CLOSE => {
                effects.push(Effect::Close);
                LRESULT(0)
            }
            accessibility::ACTIVATE_MESSAGE => {
                if let Some(element) = self.layout.focus_order().get(wparam.0).copied() {
                    self.focus = Some(element);
                    self.activate(
                        Hit {
                            element,
                            part: Part::Body,
                        },
                        &mut effects,
                    );
                    self.invalidate();
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                self.stop_recording();
                return None;
            }
            _ => return None,
        };
        Some((result, effects))
    }

    fn handle_navigation_key(&mut self, key: u16, effects: &mut Vec<Effect>) -> bool {
        let modifiers = held_modifiers();
        match VIRTUAL_KEY(key) {
            VK_TAB if modifiers & (MOD_CONTROL | MOD_ALT | MOD_WIN) == 0 => {
                self.move_focus(modifiers & MOD_SHIFT == 0);
                true
            }
            VK_SPACE | VK_RETURN if modifiers == 0 => {
                if let Some(element) = self.focus {
                    self.show_focus = true;
                    self.activate(
                        Hit {
                            element,
                            part: Part::Body,
                        },
                        effects,
                    );
                    self.invalidate();
                }
                true
            }
            VK_DELETE | VK_BACK if modifiers == 0 => {
                if let Some(Element::Recorder(action)) = self.focus
                    && self.snapshot.binding(action).is_some()
                {
                    effects.push(Effect::Clear(action));
                }
                true
            }
            VK_DOWN if modifiers == MOD_ALT && self.focus == Some(Element::RunMode) => {
                effects.push(Effect::ShowRunModeMenu);
                true
            }
            VK_F4 if modifiers == 0 && self.focus == Some(Element::RunMode) => {
                effects.push(Effect::ShowRunModeMenu);
                true
            }
            _ => false,
        }
    }

    fn move_focus(&mut self, forward: bool) {
        let order = self.layout.focus_order();
        let current = self
            .focus
            .and_then(|focus| order.iter().position(|element| *element == focus));
        let next = match (current, forward) {
            (None, true) => 0,
            (None, false) => order.len() - 1,
            (Some(index), true) => (index + 1) % order.len(),
            (Some(index), false) => (index + order.len() - 1) % order.len(),
        };
        self.focus = Some(order[next]);
        self.show_focus = true;
        self.invalidate();
    }

    fn activate(&mut self, hit: Hit, effects: &mut Vec<Effect>) {
        match (hit.element, hit.part) {
            (Element::Recorder(action), Part::Badge) => {
                if self.recording == Some(action) {
                    self.stop_recording();
                } else if self.snapshot.binding(action).is_some() {
                    effects.push(Effect::Clear(action));
                } else {
                    self.start_recording(action);
                }
            }
            (Element::Recorder(action), Part::Body) => self.start_recording(action),
            (Element::LaunchAtLogin, _) => effects.push(Effect::ToggleLaunchAtLogin),
            (Element::RunMode, _) => effects.push(Effect::ShowRunModeMenu),
            (Element::RestoreDefaults, _) => effects.push(Effect::RestoreDefaults),
        }
    }

    fn start_recording(&mut self, action: WindowAction) {
        if self.recording == Some(action) {
            return;
        }
        self.recording = Some(action);
        self.focus = Some(Element::Recorder(action));
        self.held_modifiers = held_modifiers();
        self.update_tooltips();
        self.invalidate();
    }

    fn stop_recording(&mut self) {
        if self.recording.take().is_some() {
            self.held_modifiers = 0;
            self.update_tooltips();
            self.invalidate();
        }
    }

    fn record_key(&mut self, key: u16, effects: &mut Vec<Effect>) {
        let Some(action) = self.recording else {
            return;
        };
        let modifiers = held_modifiers();
        if is_modifier_key(key) {
            if modifiers != self.held_modifiers {
                self.held_modifiers = modifiers;
                self.invalidate();
            }
            return;
        }
        if modifiers == 0 && key == VK_ESCAPE.0 {
            self.stop_recording();
            return;
        }
        if modifiers & (MOD_CONTROL | MOD_ALT | MOD_WIN) == 0 && key == VK_TAB.0 {
            self.stop_recording();
            self.move_focus(modifiers & MOD_SHIFT == 0);
            return;
        }
        let binding = KeyBinding::new(modifiers, key);
        if !binding.is_valid_shortcut() {
            return;
        }
        self.stop_recording();
        effects.push(Effect::Recorded(action, binding));
    }

    fn paint(&mut self) {
        if let Some(mut canvas) = self.canvas.take() {
            canvas.paint(self.hwnd, &self.graphics, |target| self.draw(target));
            self.canvas = Some(canvas);
        }
    }

    fn draw(&self, target: &ID2D1RenderTarget) {
        let palette = &self.palette;
        let Some(painter) = Painter::new(target, &self.graphics, self.layout.scale) else {
            return;
        };
        painter.clear(if palette.backdrop {
            D2D1_COLOR_F::default()
        } else {
            palette.solid_background
        });

        for card in &self.layout.cards {
            painter.text(
                card.title,
                &self.formats.body_strong,
                card.header,
                palette.text_primary,
            );
            painter.card(card.bounds, palette);
            for (index, row) in card.rows.iter().enumerate() {
                if index > 0 {
                    let y = row.label.top;
                    let inset = self.layout.px(CARD_PADDING);
                    painter.fill(
                        Bounds {
                            left: card.bounds.left + inset,
                            top: y,
                            right: card.bounds.right - inset,
                            bottom: y + self.layout.hairline(),
                        },
                        0.0,
                        palette.divider,
                    );
                }
                painter.text(
                    strings().action(row.action),
                    &self.formats.body,
                    row.label,
                    palette.text_primary,
                );
                self.draw_recorder(&painter, row);
            }
        }

        painter.card(self.layout.footer, palette);
        self.draw_checkbox(&painter);
        painter.text(
            strings().run_mode_label,
            &self.formats.body,
            self.layout.run_label,
            palette.text_primary,
        );
        self.draw_run_mode(&painter);
        self.draw_button(&painter);

        if self.show_focus
            && self.active
            && let Some(focus) = self.focus
        {
            let bounds = self.layout.element_bounds(focus);
            painter.focus_ring(bounds, self.layout.px(CONTROL_RADIUS), palette);
        }
    }

    fn state_of(&self, element: Element, part: Part) -> (bool, bool) {
        let target = Hit { element, part };
        let pressed = self.pressed == Some(target) && self.hover == Some(target);
        let hovered = self.hover == Some(target) && (self.pressed.is_none() || pressed);
        (hovered, pressed)
    }

    fn draw_recorder(&self, painter: &Painter, row: &Row) {
        let palette = &self.palette;
        let layout = &self.layout;
        let strings = strings();
        let element = Element::Recorder(row.action);
        let recording = self.recording == Some(row.action);
        let (hovered, pressed) = self.state_of(element, Part::Body);
        let binding = self.snapshot.binding(row.action);
        let bounds = row.recorder;
        let radius = layout.px(CONTROL_RADIUS);

        let (fill, stroke, stroke_bottom) = if recording {
            let fill = if pressed {
                palette.accent_pressed
            } else if hovered {
                palette.accent_hover
            } else {
                palette.accent
            };
            (fill, palette.accent_stroke, palette.accent_stroke_bottom)
        } else {
            let fill = if pressed {
                palette.control_fill_pressed
            } else if hovered {
                palette.control_fill_hover
            } else {
                palette.control_fill
            };
            (fill, palette.control_stroke, palette.control_stroke_bottom)
        };
        painter.fill(bounds, radius, fill);
        painter.elevation_border(bounds, radius, stroke, stroke_bottom);

        let content_left = bounds.left + layout.px(RECORDER_CONTENT_INSET);
        let badge = layout.badge(bounds);
        let unavailable =
            !recording && binding.is_some() && self.snapshot.is_unavailable(row.action);
        let content_right = if unavailable {
            layout.warning(bounds).left
        } else {
            badge.left
        } - layout.px(KEY_GAP);
        let content = Bounds {
            left: content_left,
            top: bounds.top,
            right: content_right,
            bottom: bounds.bottom,
        };

        let keys: Option<Vec<String>> = if recording {
            (self.held_modifiers != 0).then(|| {
                [MOD_WIN, MOD_CONTROL, MOD_ALT, MOD_SHIFT]
                    .into_iter()
                    .filter(|modifier| self.held_modifiers & modifier != 0)
                    .map(|modifier| modifier_label(modifier, strings.shift_key).to_owned())
                    .collect()
            })
        } else {
            binding.map(|binding| binding.display_keys(strings.shift_key))
        };
        match keys {
            Some(keys) => self.draw_keys(painter, &keys, content, recording),
            None => {
                let (text, color) = if recording {
                    (strings.recorder_enter_shortcut, palette.text_on_accent)
                } else {
                    (strings.recorder_click_to_record, palette.text_secondary)
                };
                painter.text(text, &self.formats.body, content, color);
            }
        }

        if unavailable {
            painter.text(
                GLYPH_WARNING,
                &self.formats.icon_large,
                layout.warning(bounds),
                palette.caution,
            );
        }
        let glyph = match (recording, binding.is_some()) {
            (true, true) => Some((GLYPH_UNDO, &self.formats.icon)),
            (true, false) | (false, true) => Some((GLYPH_CANCEL, &self.formats.icon_small)),
            (false, false) => None,
        };
        if let Some((glyph, format)) = glyph {
            let (badge_hovered, badge_pressed) = self.state_of(element, Part::Badge);
            let fill = match (recording, badge_pressed, badge_hovered) {
                (_, false, false) => None,
                (true, true, _) => Some(palette.key_fill_on_accent),
                (true, false, true) => Some(palette.key_fill_on_accent),
                (false, true, _) => Some(palette.subtle_fill_pressed),
                (false, false, true) => Some(palette.subtle_fill_hover),
            };
            if let Some(fill) = fill {
                painter.fill(badge, radius, fill);
            }
            let color = match (recording, badge_hovered || badge_pressed) {
                (true, true) => palette.text_on_accent,
                (true, false) => palette.text_on_accent_secondary,
                (false, true) => palette.text_primary,
                (false, false) => palette.text_secondary,
            };
            painter.text(glyph, format, badge, color);
        }
    }

    fn draw_keys(&self, painter: &Painter, keys: &[String], content: Bounds, on_accent: bool) {
        let layout = &self.layout;
        let palette = &self.palette;
        let height = layout.px(KEY_HEIGHT);
        let top = (content.top + (content.height() - height) / 2.0).round();
        let text_color = if on_accent {
            palette.text_on_accent
        } else {
            palette.text_primary
        };
        let fill = if on_accent {
            palette.key_fill_on_accent
        } else {
            palette.key_fill
        };
        let mut left = content.left;
        for key in keys {
            // Arrow keys use the Windows icon font, whose arrows share one
            // size and weight; the text font draws vertical arrows smaller.
            let (label, format) = match arrow_glyph(key) {
                Some(glyph) => (glyph, &self.formats.icon),
                None => (key.as_str(), &self.formats.key),
            };
            let width = (self.graphics.text_width(label, format).ceil()
                + 2.0 * layout.px(KEY_PADDING))
            .max(height);
            if left + width > content.right {
                let rest = Bounds {
                    left,
                    top: content.top,
                    right: content.right + layout.px(KEY_GAP),
                    bottom: content.bottom,
                };
                painter.text("\u{2026}", &self.formats.body, rest, text_color);
                break;
            }
            let bounds = Bounds::new(left, top, width, height);
            painter.fill(bounds, layout.px(CONTROL_RADIUS), fill);
            if palette.high_contrast {
                painter.stroke(bounds, layout.px(CONTROL_RADIUS), text_color);
            }
            painter.text(label, format, bounds, text_color);
            left += width + layout.px(KEY_GAP);
        }
    }

    fn draw_checkbox(&self, painter: &Painter) {
        let palette = &self.palette;
        let (hovered, pressed) = self.state_of(Element::LaunchAtLogin, Part::Body);
        painter.checkbox(
            self.layout.checkbox,
            &self.formats.icon,
            palette,
            self.snapshot.launch_at_login,
            hovered,
            pressed,
        );
        painter.text(
            strings().launch_at_login,
            &self.formats.body,
            self.layout.checkbox_label,
            palette.text_primary,
        );
    }

    fn draw_run_mode(&self, painter: &Painter) {
        let palette = &self.palette;
        let layout = &self.layout;
        let (hovered, pressed) = self.state_of(Element::RunMode, Part::Body);
        let bounds = layout.run_mode;
        painter.control_surface(bounds, palette, hovered, pressed);
        let strings = strings();
        let text = if self.snapshot.run_in_system_tray {
            strings.run_in_system_tray
        } else {
            strings.run_in_background
        };
        let text_bounds = Bounds {
            left: bounds.left + layout.px(DROPDOWN_TEXT_INSET),
            top: bounds.top,
            right: bounds.right - layout.px(DROPDOWN_CHEVRON_WIDTH),
            bottom: bounds.bottom,
        };
        painter.text(text, &self.formats.body, text_bounds, palette.text_primary);
        let chevron = Bounds {
            left: text_bounds.right,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        };
        painter.text(
            GLYPH_CHEVRON_DOWN,
            &self.formats.icon,
            chevron,
            palette.text_secondary,
        );
    }

    fn draw_button(&self, painter: &Painter) {
        let (hovered, pressed) = self.state_of(Element::RestoreDefaults, Part::Body);
        painter.button(
            self.layout.restore_defaults,
            strings().restore_defaults,
            &self.formats.body_centered,
            &self.palette,
            false,
            hovered,
            pressed,
        );
    }
}
