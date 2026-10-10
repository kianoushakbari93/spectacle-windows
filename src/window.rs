//! Windows of other applications: finding the one to act on, reading and
//! setting its bounds, and identifying the application it belongs to.

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GA_ROOT, GWL_STYLE, GetAncestor, GetClassNameW, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsHungAppWindow, IsIconic,
    IsWindow, IsWindowVisible, IsZoomed, SW_MAXIMIZE, SW_RESTORE, SWP_NOACTIVATE,
    SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos, ShowWindow, WS_CHILD, WS_THICKFRAME,
};
use windows::core::{BOOL, HSTRING, PCWSTR, w};

use crate::geometry::Rect;
use crate::movers::WindowElement;

/// Window classes of the shell itself: the desktop, taskbars, Start, search
/// and the like, which are never moved.
const SHELL_WINDOW_CLASSES: &[&str] = &[
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "NotifyIconOverflowWindow",
    "TopLevelWindowForOverflowXamlIsland",
    "Windows.UI.Core.CoreWindow",
    "XamlExplorerHostIslandWindow",
    "ForegroundStaging",
    "MultitaskingViewFrame",
    "TaskListThumbnailWnd",
    "#32768",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    hwnd: HWND,
}

impl Window {
    /// The top-level window of `hwnd`.
    pub fn from_hwnd(hwnd: HWND) -> Option<Window> {
        if hwnd.is_invalid() {
            return None;
        }
        // SAFETY: GetAncestor accepts any window handle.
        let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
        let hwnd = if root.is_invalid() { hwnd } else { root };
        Some(Window { hwnd })
    }

    /// The window the person is working in.
    pub fn foreground() -> Option<Window> {
        // SAFETY: GetForegroundWindow has no preconditions.
        Window::from_hwnd(unsafe { GetForegroundWindow() })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn class_name(&self) -> String {
        let mut buffer = [0u16; 256];
        // SAFETY: the buffer length is passed implicitly by the slice.
        let length = unsafe { GetClassNameW(self.hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..usize::try_from(length).unwrap_or(0)])
    }

    pub fn is_shell_window(&self) -> bool {
        SHELL_WINDOW_CLASSES.contains(&self.class_name().as_str())
    }

    /// Whether this is an ordinary application window that can be moved.
    pub fn is_manageable(&self) -> bool {
        // SAFETY: these functions accept any window handle.
        let visible =
            unsafe { IsWindow(Some(self.hwnd)).as_bool() && IsWindowVisible(self.hwnd).as_bool() };
        // SAFETY: as above.
        if !visible
            || unsafe { IsIconic(self.hwnd) }.as_bool()
            || self.is_cloaked()
            || self.is_shell_window()
        {
            return false;
        }
        // SAFETY: as above.
        let style = unsafe { GetWindowLongPtrW(self.hwnd, GWL_STYLE) } as u32;
        style & WS_CHILD.0 == 0
    }

    fn is_cloaked(&self) -> bool {
        let mut cloaked = 0u32;
        // SAFETY: the attribute is a DWORD and `cloaked` is one.
        let result = unsafe {
            DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_CLOAKED,
                (&mut cloaked as *mut u32).cast(),
                size_of::<u32>() as u32,
            )
        };
        result.is_ok() && cloaked != 0
    }

    fn window_rect(&self) -> Option<RECT> {
        let mut rect = RECT::default();
        // SAFETY: `rect` is a valid RECT to write to.
        unsafe { GetWindowRect(self.hwnd, &mut rect) }
            .ok()
            .map(|_| rect)
    }

    /// The bounds of the window as drawn, without the invisible resize
    /// borders Windows adds around most windows.
    fn visible_rect(&self) -> Option<RECT> {
        let mut rect = RECT::default();
        // SAFETY: the attribute is a RECT and `rect` is one.
        let result = unsafe {
            DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut rect as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            )
        };
        match result {
            Ok(()) if rect.right > rect.left && rect.bottom > rect.top => Some(rect),
            _ => self.window_rect(),
        }
    }

    /// How far the window rectangle extends beyond the visible bounds on
    /// each side.
    fn invisible_borders(&self) -> Option<RECT> {
        let window = self.window_rect()?;
        let visible = self.visible_rect()?;
        Some(RECT {
            left: visible.left - window.left,
            top: visible.top - window.top,
            right: window.right - visible.right,
            bottom: window.bottom - visible.bottom,
        })
    }

    fn place(&self, target: RECT, borders: RECT, resizable: bool) -> bool {
        let mut flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
        if !resizable {
            flags |= SWP_NOSIZE;
        }
        // SAFETY: SetWindowPos accepts any window handle; failures are reported.
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                target.left - borders.left,
                target.top - borders.top,
                (target.right - target.left) + borders.left + borders.right,
                (target.bottom - target.top) + borders.top + borders.bottom,
                flags,
            )
        }
        .is_ok()
    }

    pub fn process_id(&self) -> u32 {
        let mut process_id = 0;
        // SAFETY: `process_id` is a valid u32 to write to.
        unsafe { GetWindowThreadProcessId(self.hwnd, Some(&mut process_id)) };
        process_id
    }

    /// The application the window belongs to. Store apps are hosted by
    /// ApplicationFrameHost.exe, so for those the hosted app is looked up.
    pub fn application(&self) -> Option<Application> {
        let application = Application::from_process_id(self.process_id())?;
        if !application
            .executable_name()
            .eq_ignore_ascii_case("ApplicationFrameHost.exe")
        {
            return Some(application);
        }
        hosted_application(self.hwnd, application.process_id).or(Some(application))
    }
}

/// Finds the app a Store app frame window hosts.
fn hosted_application(frame: HWND, frame_process_id: u32) -> Option<Application> {
    struct Search {
        frame_process_id: u32,
        found: Option<u32>,
    }
    unsafe extern "system" fn visit(child: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the address of the Search below, alive for the call.
        let search = unsafe { &mut *(data.0 as *mut Search) };
        let window = Window { hwnd: child };
        let process_id = window.process_id();
        if process_id != search.frame_process_id
            && window.class_name() == "Windows.UI.Core.CoreWindow"
        {
            search.found = Some(process_id);
            return BOOL::from(false);
        }
        BOOL::from(true)
    }
    let mut search = Search {
        frame_process_id,
        found: None,
    };
    // SAFETY: the callback only uses `search`, which outlives the enumeration.
    let _ = unsafe {
        EnumChildWindows(
            Some(frame),
            Some(visit),
            LPARAM(&mut search as *mut Search as isize),
        )
    };
    Application::from_process_id(search.found?)
}

impl WindowElement for Window {
    fn rect(&self) -> Option<Rect> {
        // SAFETY: IsWindow accepts any window handle.
        if !unsafe { IsWindow(Some(self.hwnd)) }.as_bool() {
            return None;
        }
        self.visible_rect().map(|rect| Rect::from_win32(&rect))
    }

    fn set_rect(&self, rect: Rect) -> bool {
        // SAFETY: these functions accept any window handle.
        if !unsafe { IsWindow(Some(self.hwnd)) }.as_bool()
            || unsafe { IsHungAppWindow(self.hwnd) }.as_bool()
        {
            return false;
        }
        // A maximized window has to be restored before it can be placed.
        // SAFETY: as above.
        if unsafe { IsZoomed(self.hwnd) }.as_bool() {
            // SAFETY: as above.
            let _ = unsafe { ShowWindow(self.hwnd, SW_RESTORE) };
        }
        let resizable = self.is_resizable();
        let target = rect.to_win32();
        let Some(borders) = self.invisible_borders() else {
            return false;
        };
        if !self.place(target, borders, resizable) {
            return false;
        }
        // Moving to a display with a different scale changes the width of the
        // invisible borders, so place the window again with the new widths.
        if let Some(scaled_borders) = self.invisible_borders()
            && scaled_borders != borders
        {
            self.place(target, scaled_borders, resizable);
        }
        true
    }

    fn is_resizable(&self) -> bool {
        // SAFETY: GetWindowLongPtrW accepts any window handle.
        let style = unsafe { GetWindowLongPtrW(self.hwnd, GWL_STYLE) } as u32;
        style & WS_THICKFRAME.0 != 0
    }

    fn is_alive(&self) -> bool {
        // SAFETY: IsWindow accepts any window handle.
        unsafe { IsWindow(Some(self.hwnd)) }.as_bool()
    }

    fn is_maximized(&self) -> bool {
        // SAFETY: IsZoomed accepts any window handle.
        unsafe { IsZoomed(self.hwnd) }.as_bool()
    }

    fn maximize(&self) -> bool {
        if !self.is_alive() {
            return false;
        }
        // SAFETY: ShowWindow accepts any window handle.
        let _ = unsafe { ShowWindow(self.hwnd, SW_MAXIMIZE) };
        self.is_maximized()
    }
}

/// A running application, identified by its executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Application {
    pub process_id: u32,
    pub path: PathBuf,
}

impl Application {
    pub fn from_process_id(process_id: u32) -> Option<Application> {
        // SAFETY: OpenProcess with query-only access; the handle is closed below.
        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
        let mut buffer = vec![0u16; 32_768];
        let mut length = buffer.len() as u32;
        // SAFETY: `length` holds the buffer's capacity in UTF-16 code units.
        let result = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        };
        // SAFETY: the handle came from OpenProcess and is closed exactly once.
        let _ = unsafe { CloseHandle(process) };
        result.ok()?;
        let path = PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize]));
        Some(Application { process_id, path })
    }

    /// The executable's file name, e.g. `notepad.exe`.
    pub fn executable_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// A case-insensitive key identifying the application across processes.
    pub fn key(&self) -> String {
        self.path.to_string_lossy().to_lowercase()
    }

    /// The application's name as its publisher describes it, e.g. "Notepad".
    pub fn display_name(&self) -> String {
        file_description(&self.path)
            .filter(|description| !description.trim().is_empty())
            .unwrap_or_else(|| {
                self.path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
    }
}

fn file_description(path: &Path) -> Option<String> {
    let path = HSTRING::from(path.as_os_str());
    // SAFETY: the path is a valid null-terminated string.
    let size = unsafe { GetFileVersionInfoSizeW(&path, None) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    // SAFETY: `data` has the size GetFileVersionInfoSizeW asked for.
    unsafe { GetFileVersionInfoW(&path, None, size, data.as_mut_ptr().cast()) }.ok()?;
    let translation = query_version_value(&data, w!("\\VarFileInfo\\Translation"))?;
    let mut languages: Vec<(u16, u16)> = translation
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pair| {
            (
                u16::from_le_bytes([pair[0], pair[1]]),
                u16::from_le_bytes([pair[2], pair[3]]),
            )
        })
        .collect();
    languages.push((0x0409, 0x04B0));
    languages.into_iter().find_map(|(language, code_page)| {
        let key = HSTRING::from(format!(
            "\\StringFileInfo\\{language:04x}{code_page:04x}\\FileDescription"
        ));
        let value = query_version_value(&data, PCWSTR(key.as_ptr()))?;
        let units: Vec<u16> = value
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes(*unit))
            .take_while(|unit| *unit != 0)
            .collect();
        Some(String::from_utf16_lossy(&units))
    })
}

fn query_version_value(data: &[u8], key: PCWSTR) -> Option<&[u8]> {
    let mut pointer = std::ptr::null_mut();
    let mut length = 0u32;
    // SAFETY: `data` holds a version resource; the returned pointer points into it.
    if !unsafe { VerQueryValueW(data.as_ptr().cast(), key, &mut pointer, &mut length) }.as_bool()
        || pointer.is_null()
    {
        return None;
    }
    let start = (pointer as usize).checked_sub(data.as_ptr() as usize)?;
    // String values report their length in characters, other values in bytes.
    let byte_length = if key_is_string(key) {
        length as usize * 2
    } else {
        length as usize
    };
    let end = start.checked_add(byte_length)?.min(data.len());
    data.get(start..end)
}

fn key_is_string(key: PCWSTR) -> bool {
    // SAFETY: every key passed in is a valid null-terminated string.
    unsafe { key.to_string() }.is_ok_and(|key| key.starts_with("\\StringFileInfo"))
}
