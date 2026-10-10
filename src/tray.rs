//! Spectacle's icon in the system tray.

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetSystemMetricsForDpi, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON, SM_CXSMICON};

use crate::graphics::Graphics;
use crate::theme;

const ICON_ID: u32 = 1;

pub struct TrayIcon {
    hwnd: HWND,
    callback_message: u32,
    icon: Option<HICON>,
    added: bool,
}

impl TrayIcon {
    pub fn new(hwnd: HWND, callback_message: u32) -> TrayIcon {
        TrayIcon {
            hwnd,
            callback_message,
            icon: None,
            added: false,
        }
    }

    /// Adds the icon, or adds it again after Explorer restarts.
    pub fn show(&mut self) {
        self.render_icon();
        let mut data = self.data();
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
        // SAFETY: `data` is a fully initialised NOTIFYICONDATAW.
        unsafe {
            if !Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
                // The icon may survive from before Explorer restarted.
                let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
            }
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
        }
        self.added = true;
    }

    pub fn hide(&mut self) {
        if self.added {
            let data = self.data();
            // SAFETY: removes the icon this value added.
            let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
            self.added = false;
        }
        self.destroy_icon();
    }

    /// Redraws the icon for the current taskbar theme and display scale.
    pub fn refresh(&mut self) {
        if !self.added {
            return;
        }
        self.render_icon();
        let mut data = self.data();
        data.uFlags = NIF_ICON;
        // SAFETY: `data` is a fully initialised NOTIFYICONDATAW.
        let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
    }

    fn data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: ICON_ID,
            uCallbackMessage: self.callback_message,
            hIcon: self.icon.unwrap_or_default(),
            ..Default::default()
        };
        let tip = format!("Spectacle {}", crate::VERSION);
        for (slot, unit) in data.szTip.iter_mut().zip(tip.encode_utf16().take(127)) {
            *slot = unit;
        }
        data
    }

    fn render_icon(&mut self) {
        // The taskbar lives on the primary display and follows its scale.
        // SAFETY: plain queries with valid out parameters.
        let size = unsafe {
            let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let (mut dpi_x, mut dpi_y) = (96, 96);
            let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            GetSystemMetricsForDpi(SM_CXSMICON, dpi_x)
        };
        let color = if theme::is_high_contrast() {
            // In high contrast the taskbar uses the window text colour.
            let palette = theme::Palette::current();
            palette.text_primary
        } else if theme::taskbar_uses_light_theme() {
            D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }
        } else {
            D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }
        };
        let icon = Graphics::new().and_then(|graphics| graphics.tray_icon(size, color));
        if let Ok(icon) = icon {
            self.destroy_icon();
            self.icon = Some(icon);
        }
    }

    fn destroy_icon(&mut self) {
        if let Some(icon) = self.icon.take() {
            // SAFETY: the icon was created by this value.
            let _ = unsafe { DestroyIcon(icon) };
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.hide();
    }
}
