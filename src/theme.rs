//! Light, dark and high contrast appearance, following the Windows 11
//! (Fluent) design tokens.

use windows::UI::Color;
use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Gdi::{
    COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW,
    COLOR_WINDOWTEXT, GetSysColor, SYS_COLOR_INDEX,
};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::{PCSTR, w};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub high_contrast: bool,
    /// Whether the window can show the Mica backdrop behind its content.
    pub backdrop: bool,
    pub solid_background: D2D1_COLOR_F,
    pub text_primary: D2D1_COLOR_F,
    pub text_secondary: D2D1_COLOR_F,
    pub text_on_accent: D2D1_COLOR_F,
    pub text_on_accent_secondary: D2D1_COLOR_F,
    pub card_fill: D2D1_COLOR_F,
    pub card_stroke: D2D1_COLOR_F,
    pub divider: D2D1_COLOR_F,
    pub control_fill: D2D1_COLOR_F,
    pub control_fill_hover: D2D1_COLOR_F,
    pub control_fill_pressed: D2D1_COLOR_F,
    pub control_stroke: D2D1_COLOR_F,
    pub control_stroke_bottom: D2D1_COLOR_F,
    pub control_strong_stroke: D2D1_COLOR_F,
    pub control_alt_fill: D2D1_COLOR_F,
    pub control_alt_fill_hover: D2D1_COLOR_F,
    pub control_alt_fill_pressed: D2D1_COLOR_F,
    pub subtle_fill_hover: D2D1_COLOR_F,
    pub subtle_fill_pressed: D2D1_COLOR_F,
    pub key_fill: D2D1_COLOR_F,
    pub key_fill_on_accent: D2D1_COLOR_F,
    pub accent: D2D1_COLOR_F,
    pub accent_hover: D2D1_COLOR_F,
    pub accent_pressed: D2D1_COLOR_F,
    pub accent_stroke: D2D1_COLOR_F,
    pub accent_stroke_bottom: D2D1_COLOR_F,
    pub caution: D2D1_COLOR_F,
    pub focus_outer: D2D1_COLOR_F,
    pub focus_inner: D2D1_COLOR_F,
}

const fn argb(value: u32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        a: ((value >> 24) & 0xFF) as f32 / 255.0,
        r: ((value >> 16) & 0xFF) as f32 / 255.0,
        g: ((value >> 8) & 0xFF) as f32 / 255.0,
        b: (value & 0xFF) as f32 / 255.0,
    }
}

fn from_color(color: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        a: f32::from(color.A) / 255.0,
        r: f32::from(color.R) / 255.0,
        g: f32::from(color.G) / 255.0,
        b: f32::from(color.B) / 255.0,
    }
}

fn with_alpha(color: D2D1_COLOR_F, alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { a: alpha, ..color }
}

fn system_color(index: SYS_COLOR_INDEX) -> D2D1_COLOR_F {
    // SAFETY: GetSysColor has no preconditions.
    let value = unsafe { GetSysColor(index) };
    D2D1_COLOR_F {
        r: (value & 0xFF) as f32 / 255.0,
        g: ((value >> 8) & 0xFF) as f32 / 255.0,
        b: ((value >> 16) & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

pub fn is_high_contrast() -> bool {
    let mut contrast = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    // SAFETY: `contrast` is a properly sized HIGHCONTRASTW.
    let result = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    result.is_ok() && contrast.dwFlags.contains(HCF_HIGHCONTRASTON)
}

/// Whether apps should use dark mode, using the method Microsoft documents:
/// a light foreground colour means a dark theme.
pub fn apps_use_dark_mode() -> bool {
    UISettings::new()
        .and_then(|settings| settings.GetColorValue(UIColorType::Foreground))
        .map(|color| {
            (5 * u32::from(color.G) + 2 * u32::from(color.R) + u32::from(color.B)) > 8 * 128
        })
        .unwrap_or_else(|_| {
            read_dword(
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("AppsUseLightTheme"),
            ) == Some(0)
        })
}

/// Whether the taskbar is light, so the tray icon should be dark.
pub fn taskbar_uses_light_theme() -> bool {
    read_dword(
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
        w!("SystemUsesLightTheme"),
    )
    .is_some_and(|value| value != 0)
}

fn transparency_effects_enabled() -> bool {
    UISettings::new()
        .and_then(|settings| settings.AdvancedEffectsEnabled())
        .unwrap_or(false)
}

pub fn windows_build_number() -> u32 {
    let mut buffer = [0u16; 16];
    let mut size = size_of_val(&buffer) as u32;
    // SAFETY: `buffer` holds `size` bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.is_err() {
        return 0;
    }
    String::from_utf16_lossy(&buffer[..(size as usize / 2).saturating_sub(1)])
        .trim()
        .parse()
        .unwrap_or(0)
}

fn read_dword(key: windows::core::PCWSTR, value: windows::core::PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `data` holds the DWORD the value is read into.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    };
    status.is_ok().then_some(data)
}

/// Lets popup menus follow the dark or light app mode, as the menus of
/// Windows' own notification area icons do.
pub fn allow_dark_menus() {
    // uxtheme's SetPreferredAppMode (ordinal 135) and FlushMenuThemes
    // (ordinal 136) have had these ordinals since Windows 10 version 1903
    // (build 18362).
    if windows_build_number() < 18362 {
        return;
    }
    // SAFETY: uxtheme.dll is loaded from System32 only; the functions are
    // looked up by their stable ordinals and called with their signatures.
    unsafe {
        let Ok(uxtheme) = LoadLibraryExW(w!("uxtheme.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
        else {
            return;
        };
        let set_preferred_app_mode = GetProcAddress(uxtheme, PCSTR(135 as *const u8));
        let flush_menu_themes = GetProcAddress(uxtheme, PCSTR(136 as *const u8));
        if let (Some(set_preferred_app_mode), Some(flush_menu_themes)) =
            (set_preferred_app_mode, flush_menu_themes)
        {
            const ALLOW_DARK: i32 = 1;
            let set_preferred_app_mode: unsafe extern "system" fn(i32) -> i32 =
                std::mem::transmute(set_preferred_app_mode);
            let flush_menu_themes: unsafe extern "system" fn() =
                std::mem::transmute(flush_menu_themes);
            set_preferred_app_mode(ALLOW_DARK);
            flush_menu_themes();
        }
    }
}

impl Palette {
    pub fn current() -> Palette {
        if is_high_contrast() {
            return Palette::high_contrast();
        }
        let dark = apps_use_dark_mode();
        // Development builds can preview the other theme without changing
        // the system's.
        #[cfg(debug_assertions)]
        let dark = match std::env::var("SPECTACLE_DEBUG_THEME").as_deref() {
            Ok("light") => false,
            Ok("dark") => true,
            _ => dark,
        };
        let settings = UISettings::new().ok();
        let accent_color = |kind: UIColorType, fallback: u32| {
            settings
                .as_ref()
                .and_then(|settings| settings.GetColorValue(kind).ok())
                .map(from_color)
                .unwrap_or(argb(fallback))
        };
        let backdrop = transparency_effects_enabled() && windows_build_number() >= 22621;
        if dark {
            let accent = accent_color(UIColorType::AccentLight2, 0xFF99EBFF);
            Palette {
                dark,
                high_contrast: false,
                backdrop,
                solid_background: argb(0xFF202020),
                text_primary: argb(0xFFFFFFFF),
                text_secondary: argb(0xC5FFFFFF),
                text_on_accent: argb(0xFF000000),
                text_on_accent_secondary: argb(0x80000000),
                card_fill: argb(0x0DFFFFFF),
                card_stroke: argb(0x19000000),
                divider: argb(0x15FFFFFF),
                control_fill: argb(0x0FFFFFFF),
                control_fill_hover: argb(0x15FFFFFF),
                control_fill_pressed: argb(0x08FFFFFF),
                control_stroke: argb(0x12FFFFFF),
                control_stroke_bottom: argb(0x18FFFFFF),
                control_strong_stroke: argb(0x8BFFFFFF),
                control_alt_fill: argb(0x19000000),
                control_alt_fill_hover: argb(0x0BFFFFFF),
                control_alt_fill_pressed: argb(0x12FFFFFF),
                subtle_fill_hover: argb(0x0FFFFFFF),
                subtle_fill_pressed: argb(0x0AFFFFFF),
                key_fill: argb(0x1AFFFFFF),
                key_fill_on_accent: argb(0x24000000),
                accent,
                accent_hover: with_alpha(accent, 0.9),
                accent_pressed: with_alpha(accent, 0.8),
                accent_stroke: argb(0x14FFFFFF),
                accent_stroke_bottom: argb(0x23000000),
                caution: argb(0xFFFCE100),
                focus_outer: argb(0xFFFFFFFF),
                focus_inner: argb(0xB3000000),
            }
        } else {
            let accent = accent_color(UIColorType::AccentDark1, 0xFF005FB8);
            Palette {
                dark,
                high_contrast: false,
                backdrop,
                solid_background: argb(0xFFF3F3F3),
                text_primary: argb(0xE4000000),
                text_secondary: argb(0x9E000000),
                text_on_accent: argb(0xFFFFFFFF),
                text_on_accent_secondary: argb(0xB3FFFFFF),
                card_fill: argb(0xB3FFFFFF),
                card_stroke: argb(0x0F000000),
                divider: argb(0x0F000000),
                control_fill: argb(0xB3FFFFFF),
                control_fill_hover: argb(0x80F9F9F9),
                control_fill_pressed: argb(0x4DF9F9F9),
                control_stroke: argb(0x0F000000),
                control_stroke_bottom: argb(0x29000000),
                control_strong_stroke: argb(0x72000000),
                control_alt_fill: argb(0x06000000),
                control_alt_fill_hover: argb(0x0F000000),
                control_alt_fill_pressed: argb(0x18000000),
                subtle_fill_hover: argb(0x09000000),
                subtle_fill_pressed: argb(0x06000000),
                key_fill: argb(0x0F000000),
                key_fill_on_accent: argb(0x33FFFFFF),
                accent,
                accent_hover: with_alpha(accent, 0.9),
                accent_pressed: with_alpha(accent, 0.8),
                accent_stroke: argb(0x14FFFFFF),
                accent_stroke_bottom: argb(0x66000000),
                caution: argb(0xFF9D5D00),
                focus_outer: argb(0xE4000000),
                focus_inner: argb(0xB3FFFFFF),
            }
        }
    }

    /// The person's high contrast theme colours, used without transparency.
    fn high_contrast() -> Palette {
        let window = system_color(COLOR_WINDOW);
        let text = system_color(COLOR_WINDOWTEXT);
        let highlight = system_color(COLOR_HIGHLIGHT);
        let highlight_text = system_color(COLOR_HIGHLIGHTTEXT);
        let gray_text = system_color(COLOR_GRAYTEXT);
        let button_text = system_color(COLOR_BTNTEXT);
        let dark = (window.r * 0.2126 + window.g * 0.7152 + window.b * 0.0722) < 0.5;
        Palette {
            dark,
            high_contrast: true,
            backdrop: false,
            solid_background: window,
            text_primary: text,
            text_secondary: gray_text,
            text_on_accent: highlight_text,
            text_on_accent_secondary: highlight_text,
            card_fill: window,
            card_stroke: text,
            divider: text,
            control_fill: window,
            control_fill_hover: window,
            control_fill_pressed: window,
            control_stroke: button_text,
            control_stroke_bottom: button_text,
            control_strong_stroke: button_text,
            control_alt_fill: window,
            control_alt_fill_hover: window,
            control_alt_fill_pressed: window,
            subtle_fill_hover: window,
            subtle_fill_pressed: window,
            key_fill: window,
            key_fill_on_accent: highlight,
            accent: highlight,
            accent_hover: highlight,
            accent_pressed: highlight,
            accent_stroke: highlight_text,
            accent_stroke_bottom: highlight_text,
            caution: text,
            focus_outer: text,
            focus_inner: window,
        }
    }
}
