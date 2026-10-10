//! Drawing shared by Spectacle's windows: an offscreen canvas, pixel-snapped
//! shapes and the Windows 11 controls the Settings window and dialogs use.

use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D1_GRADIENT_STOP};
use windows::Win32::Graphics::Direct2D::{
    D2D1_EXTEND_MODE_CLAMP, D2D1_GAMMA_2_2, D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES,
    D2D1_ROUNDED_RECT, ID2D1DCRenderTarget, ID2D1RenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;
use windows::Win32::Graphics::Dwm::{
    DWM_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{BeginPaint, BitBlt, EndPaint, PAINTSTRUCT, SRCCOPY};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{LoadIconWithScaleDown, MARGINS};
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetClientRect, HICON, ICON_BIG, ICON_SMALL, SM_CXICON, SM_CXSMICON, SendMessageW,
    WM_SETICON,
};
use windows::core::BOOL;
use windows_numerics::Vector2;

use crate::dialogs::APP_ICON_RESOURCE;
use crate::graphics::{DibSurface, Graphics, draw_text, rect_f};
use crate::theme::Palette;

/// The corner radius of controls, in device independent pixels.
pub const CONTROL_RADIUS: f32 = 4.0;
/// The corner radius of cards, in device independent pixels.
pub const CARD_RADIUS: f32 = 8.0;

/// Segoe Fluent Icons check mark.
pub const GLYPH_CHECK: &str = "\u{E73E}";

/// A rectangle in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Bounds {
    pub fn new(left: f32, top: f32, width: f32, height: f32) -> Bounds {
        Bounds {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    pub fn inflated(&self, amount: f32) -> Bounds {
        Bounds {
            left: self.left - amount,
            top: self.top - amount,
            right: self.right + amount,
            bottom: self.bottom + amount,
        }
    }

    pub fn to_rect(self) -> RECT {
        RECT {
            left: self.left as i32,
            top: self.top as i32,
            right: self.right as i32,
            bottom: self.bottom as i32,
        }
    }
}

/// Lets the window follow the dark or light app mode in its title bar and
/// show the Mica backdrop behind its content where available.
pub fn apply_window_appearance(hwnd: HWND, palette: &Palette) {
    let dark = BOOL::from(palette.dark);
    let backdrop: DWM_SYSTEMBACKDROP_TYPE = if palette.backdrop {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };
    let margins = if palette.backdrop {
        MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        }
    } else {
        MARGINS::default()
    };
    // SAFETY: the attributes are passed with their documented types.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark as *const BOOL).cast(),
            size_of::<BOOL>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&backdrop as *const DWM_SYSTEMBACKDROP_TYPE).cast(),
            size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }
}

/// The window's title bar and taskbar icons, at the window's scale.
#[derive(Default)]
pub struct WindowIcons {
    icons: Vec<HICON>,
}

impl WindowIcons {
    pub fn set(&mut self, hwnd: HWND, dpi: u32) {
        self.destroy();
        // SAFETY: loads this executable's icon at the window's DPI sizes.
        unsafe {
            let Ok(module) = GetModuleHandleW(None) else {
                return;
            };
            for (kind, metric) in [(ICON_SMALL, SM_CXSMICON), (ICON_BIG, SM_CXICON)] {
                let size = GetSystemMetricsForDpi(metric, dpi);
                if let Ok(icon) =
                    LoadIconWithScaleDown(Some(module.into()), APP_ICON_RESOURCE, size, size)
                {
                    SendMessageW(
                        hwnd,
                        WM_SETICON,
                        Some(WPARAM(kind as usize)),
                        Some(LPARAM(icon.0 as isize)),
                    );
                    self.icons.push(icon);
                }
            }
        }
    }

    fn destroy(&mut self) {
        for icon in self.icons.drain(..) {
            // SAFETY: the icons were loaded by this value.
            let _ = unsafe { DestroyIcon(icon) };
        }
    }
}

impl Drop for WindowIcons {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// Paints a window by drawing into an offscreen bitmap, which keeps the
/// alpha channel the Mica backdrop shows through.
pub struct Canvas {
    target: ID2D1DCRenderTarget,
    surface: Option<DibSurface>,
}

impl Canvas {
    pub fn new(graphics: &Graphics) -> windows::core::Result<Canvas> {
        Ok(Canvas {
            target: graphics.dc_render_target()?,
            surface: None,
        })
    }

    /// Discards the bitmap, for example after the window changes size.
    pub fn reset(&mut self) {
        self.surface = None;
    }

    /// Handles WM_PAINT by calling `draw` with the render target.
    pub fn paint(
        &mut self,
        hwnd: HWND,
        graphics: &Graphics,
        draw: impl FnOnce(&ID2D1RenderTarget),
    ) {
        let mut paint = PAINTSTRUCT::default();
        // SAFETY: paints this thread's window.
        let dc = unsafe { BeginPaint(hwnd, &mut paint) };
        let mut client = RECT::default();
        // SAFETY: as above.
        let _ = unsafe { GetClientRect(hwnd, &mut client) };
        let (width, height) = (client.right - client.left, client.bottom - client.top);
        if self
            .surface
            .as_ref()
            .is_none_or(|s| s.width != width || s.height != height)
        {
            self.surface = DibSurface::new(width, height).ok();
        }
        if let Some(surface) = &self.surface {
            // SAFETY: draws into the surface's DC and copies it to the window.
            unsafe {
                if self.target.BindDC(surface.dc, &client).is_ok() {
                    self.target.BeginDraw();
                    draw(&self.target);
                    if self.target.EndDraw(None, None).is_err()
                        && let Ok(target) = graphics.dc_render_target()
                    {
                        // The target is recreated after a device error.
                        self.target = target;
                    }
                    let _ = BitBlt(dc, 0, 0, width, height, Some(surface.dc), 0, 0, SRCCOPY);
                }
            }
        }
        // SAFETY: ends the paint started above.
        let _ = unsafe { EndPaint(hwnd, &paint) };
    }
}

/// Drawing helpers working in physical pixels.
pub struct Painter<'a> {
    target: &'a ID2D1RenderTarget,
    brush: ID2D1SolidColorBrush,
    graphics: &'a Graphics,
    scale: f32,
}

impl<'a> Painter<'a> {
    /// A painter for a target between BeginDraw and EndDraw.
    pub fn new(
        target: &'a ID2D1RenderTarget,
        graphics: &'a Graphics,
        scale: f32,
    ) -> Option<Painter<'a>> {
        // SAFETY: plain resource creation on a target in BeginDraw.
        let brush = unsafe { target.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None) }.ok()?;
        Some(Painter {
            target,
            brush,
            graphics,
            scale,
        })
    }

    /// Device independent pixels in whole physical pixels.
    pub fn px(&self, dips: f32) -> f32 {
        (dips * self.scale).round()
    }

    /// One device independent pixel, at least one physical pixel.
    pub fn hairline(&self) -> f32 {
        self.px(1.0).max(1.0)
    }

    pub fn clear(&self, color: D2D1_COLOR_F) {
        // SAFETY: drawing between BeginDraw and EndDraw.
        unsafe { self.target.Clear(Some(&color)) };
    }

    fn rounded(bounds: Bounds, radius: f32) -> D2D1_ROUNDED_RECT {
        D2D1_ROUNDED_RECT {
            rect: rect_f(bounds.left, bounds.top, bounds.right, bounds.bottom),
            radiusX: radius,
            radiusY: radius,
        }
    }

    pub fn fill(&self, bounds: Bounds, radius: f32, color: D2D1_COLOR_F) {
        // SAFETY: drawing between BeginDraw and EndDraw.
        unsafe {
            self.brush.SetColor(&color);
            if radius > 0.0 {
                self.target
                    .FillRoundedRectangle(&Self::rounded(bounds, radius), &self.brush);
            } else {
                self.target.FillRectangle(
                    &rect_f(bounds.left, bounds.top, bounds.right, bounds.bottom),
                    &self.brush,
                );
            }
        }
    }

    /// Strokes a hairline just inside `bounds`.
    pub fn stroke(&self, bounds: Bounds, radius: f32, color: D2D1_COLOR_F) {
        let width = self.hairline();
        let inset = width / 2.0;
        // SAFETY: as above.
        unsafe {
            self.brush.SetColor(&color);
            self.target.DrawRoundedRectangle(
                &Self::rounded(bounds.inflated(-inset), (radius - inset).max(0.0)),
                &self.brush,
                width,
                None,
            );
        }
    }

    /// The Windows 11 control border: darker along the bottom edge, which
    /// gives controls their slight elevation.
    pub fn elevation_border(
        &self,
        bounds: Bounds,
        radius: f32,
        color: D2D1_COLOR_F,
        bottom_color: D2D1_COLOR_F,
    ) {
        let width = self.hairline();
        let inset = width / 2.0;
        let stops = [
            D2D1_GRADIENT_STOP {
                position: 0.33,
                color: bottom_color,
            },
            D2D1_GRADIENT_STOP {
                position: 1.0,
                color,
            },
        ];
        // SAFETY: as above.
        unsafe {
            let Ok(collection) = self.target.CreateGradientStopCollection(
                &stops,
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            ) else {
                return;
            };
            let properties = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                startPoint: Vector2 {
                    X: 0.0,
                    Y: bounds.bottom,
                },
                endPoint: Vector2 {
                    X: 0.0,
                    Y: bounds.bottom - self.px(3.0),
                },
            };
            if let Ok(brush) = self
                .target
                .CreateLinearGradientBrush(&properties, None, &collection)
            {
                self.target.DrawRoundedRectangle(
                    &Self::rounded(bounds.inflated(-inset), (radius - inset).max(0.0)),
                    &brush,
                    width,
                    None,
                );
            }
        }
    }

    pub fn card(&self, bounds: Bounds, palette: &Palette) {
        let radius = self.px(CARD_RADIUS);
        self.fill(bounds, radius, palette.card_fill);
        self.stroke(bounds, radius, palette.card_stroke);
    }

    /// Draws text laid out within `bounds` with the format's alignment.
    pub fn text(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        bounds: Bounds,
        color: D2D1_COLOR_F,
    ) {
        if let Ok(layout) = self
            .graphics
            .layout(text, format, bounds.width(), bounds.height())
        {
            draw_text(self.target, &layout, bounds.left, bounds.top, color);
        }
    }

    /// The Windows 11 keyboard focus indicator: a two pixel outer ring and a
    /// one pixel inner ring around the control.
    pub fn focus_ring(&self, bounds: Bounds, radius: f32, palette: &Palette) {
        let outer_width = self.px(2.0).max(2.0);
        let inner_width = self.hairline();
        let margin = self.px(3.0);
        let outer = bounds.inflated(margin - outer_width / 2.0);
        let inner = bounds.inflated(margin - outer_width - inner_width / 2.0);
        // SAFETY: as above.
        unsafe {
            self.brush.SetColor(&palette.focus_outer);
            self.target.DrawRoundedRectangle(
                &Self::rounded(outer, radius + margin - outer_width / 2.0),
                &self.brush,
                outer_width,
                None,
            );
            self.brush.SetColor(&palette.focus_inner);
            self.target.DrawRoundedRectangle(
                &Self::rounded(
                    inner,
                    (radius + margin - outer_width - inner_width / 2.0).max(0.0),
                ),
                &self.brush,
                inner_width,
                None,
            );
        }
    }

    /// The surface of a standard button or drop-down.
    pub fn control_surface(&self, bounds: Bounds, palette: &Palette, hovered: bool, pressed: bool) {
        let radius = self.px(CONTROL_RADIUS);
        let fill = if pressed {
            palette.control_fill_pressed
        } else if hovered {
            palette.control_fill_hover
        } else {
            palette.control_fill
        };
        self.fill(bounds, radius, fill);
        if pressed {
            self.stroke(bounds, radius, palette.control_stroke);
        } else {
            self.elevation_border(
                bounds,
                radius,
                palette.control_stroke,
                palette.control_stroke_bottom,
            );
        }
    }

    /// A push button; the accent button is the default one of a dialog.
    #[allow(clippy::too_many_arguments)]
    pub fn button(
        &self,
        bounds: Bounds,
        label: &str,
        format: &IDWriteTextFormat,
        palette: &Palette,
        accent: bool,
        hovered: bool,
        pressed: bool,
    ) {
        let color = if accent {
            let radius = self.px(CONTROL_RADIUS);
            let fill = if pressed {
                palette.accent_pressed
            } else if hovered {
                palette.accent_hover
            } else {
                palette.accent
            };
            self.fill(bounds, radius, fill);
            if !pressed {
                self.elevation_border(
                    bounds,
                    radius,
                    palette.accent_stroke,
                    palette.accent_stroke_bottom,
                );
            }
            if pressed {
                palette.text_on_accent_secondary
            } else {
                palette.text_on_accent
            }
        } else {
            self.control_surface(bounds, palette, hovered, pressed);
            if pressed {
                palette.text_secondary
            } else {
                palette.text_primary
            }
        };
        self.text(label, format, bounds, color);
    }

    /// The box of a check box; its label is drawn separately.
    pub fn checkbox(
        &self,
        bounds: Bounds,
        check: &IDWriteTextFormat,
        palette: &Palette,
        checked: bool,
        hovered: bool,
        pressed: bool,
    ) {
        let radius = self.px(CONTROL_RADIUS);
        if checked {
            let fill = if pressed {
                palette.accent_pressed
            } else if hovered {
                palette.accent_hover
            } else {
                palette.accent
            };
            self.fill(bounds, radius, fill);
            self.text(GLYPH_CHECK, check, bounds, palette.text_on_accent);
        } else {
            let fill = if pressed {
                palette.control_alt_fill_pressed
            } else if hovered {
                palette.control_alt_fill_hover
            } else {
                palette.control_alt_fill
            };
            self.fill(bounds, radius, fill);
            self.stroke(bounds, radius, palette.control_strong_stroke);
        }
    }
}
