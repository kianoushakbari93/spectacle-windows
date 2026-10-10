//! Drawing with Direct2D and DirectWrite.
//!
//! Rendering is done in software into a 32-bit premultiplied alpha bitmap.
//! The windows Spectacle draws are small and change rarely, and software
//! rendering keeps graphics drivers out of the process, which keeps memory
//! use low.

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_PROPERTIES, D2D1_CAP_STYLE_ROUND, D2D1_DRAW_TEXT_OPTIONS_CLIP,
    D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_LINE_JOIN_ROUND, D2D1_QUADRATIC_BEZIER_SEGMENT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_STROKE_STYLE_PROPERTIES,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1Bitmap, ID2D1DCRenderTarget,
    ID2D1Factory, ID2D1RenderTarget, ID2D1StrokeStyle,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
    DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_METRICS, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP, DWRITE_WORD_WRAPPING_WRAP,
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
    HGDIOBJ, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, GetIconInfo, HICON, ICONINFO};
use windows::core::{HSTRING, Result};
use windows_numerics::Vector2;

pub struct Graphics {
    pub factory: ID2D1Factory,
    pub dwrite: IDWriteFactory,
    pub text_family: HSTRING,
    pub icon_family: HSTRING,
    round_stroke: ID2D1StrokeStyle,
}

impl Graphics {
    pub fn new() -> Result<Graphics> {
        // SAFETY: plain factory creation.
        let factory: ID2D1Factory = unsafe {
            D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                Some(&D2D1_FACTORY_OPTIONS::default()),
            )?
        };
        // SAFETY: plain factory creation.
        let dwrite: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let text_family = first_installed_family(&dwrite, &["Segoe UI Variable Text", "Segoe UI"]);
        let icon_family =
            first_installed_family(&dwrite, &["Segoe Fluent Icons", "Segoe MDL2 Assets"]);
        let properties = D2D1_STROKE_STYLE_PROPERTIES {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            ..Default::default()
        };
        // SAFETY: plain resource creation.
        let round_stroke = unsafe { factory.CreateStrokeStyle(&properties, None)? };
        Ok(Graphics {
            factory,
            dwrite,
            text_family,
            icon_family,
            round_stroke,
        })
    }

    pub fn text_format(
        &self,
        size: f32,
        weight: DWRITE_FONT_WEIGHT,
        alignment: DWRITE_TEXT_ALIGNMENT,
    ) -> Result<IDWriteTextFormat> {
        self.format(&self.text_family, size, weight, alignment)
    }

    pub fn icon_format(&self, size: f32) -> Result<IDWriteTextFormat> {
        self.format(
            &self.icon_family,
            size,
            DWRITE_FONT_WEIGHT(400),
            windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT_CENTER,
        )
    }

    fn format(
        &self,
        family: &HSTRING,
        size: f32,
        weight: DWRITE_FONT_WEIGHT,
        alignment: DWRITE_TEXT_ALIGNMENT,
    ) -> Result<IDWriteTextFormat> {
        // SAFETY: plain resource creation with valid strings.
        unsafe {
            let format = self.dwrite.CreateTextFormat(
                family,
                None,
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                &HSTRING::from("en-us"),
            )?;
            format.SetTextAlignment(alignment)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                ..Default::default()
            };
            let sign = self.dwrite.CreateEllipsisTrimmingSign(&format)?;
            format.SetTrimming(&trimming, &sign)?;
            Ok(format)
        }
    }

    pub fn layout(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        width: f32,
        height: f32,
    ) -> Result<IDWriteTextLayout> {
        let text: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: plain resource creation.
        unsafe {
            self.dwrite
                .CreateTextLayout(&text, format, width.max(0.0), height.max(0.0))
        }
    }

    /// A format for text that wraps onto as many lines as it needs.
    pub fn paragraph_format(
        &self,
        size: f32,
        weight: DWRITE_FONT_WEIGHT,
    ) -> Result<IDWriteTextFormat> {
        let format = self.format(
            &self.text_family,
            size,
            weight,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        // SAFETY: configures the format just created.
        unsafe {
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
            format.SetTrimming(&DWRITE_TRIMMING::default(), None)?;
        }
        Ok(format)
    }

    /// The height of `text` wrapped to `width`.
    pub fn text_height(&self, text: &str, format: &IDWriteTextFormat, width: f32) -> f32 {
        self.layout(text, format, width, 100_000.0)
            .and_then(|layout| {
                let mut metrics = DWRITE_TEXT_METRICS::default();
                // SAFETY: `metrics` is a valid out parameter.
                unsafe { layout.GetMetrics(&mut metrics) }.map(|_| metrics.height)
            })
            .unwrap_or(0.0)
    }

    /// An icon as a bitmap `target` can draw.
    pub fn icon_bitmap(&self, target: &ID2D1RenderTarget, icon: HICON) -> Result<ID2D1Bitmap> {
        let mut info = ICONINFO::default();
        // SAFETY: GetIconInfo creates two bitmaps, which are deleted below.
        unsafe { GetIconInfo(icon, &mut info)? };
        let result = (|| {
            let mut bitmap = BITMAP::default();
            // SAFETY: reads the colour bitmap's header into `bitmap`.
            if unsafe {
                GetObjectW(
                    info.hbmColor.into(),
                    size_of::<BITMAP>() as i32,
                    Some((&mut bitmap as *mut BITMAP).cast()),
                )
            } == 0
            {
                return Err(windows::core::Error::from_thread());
            }
            let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
            let mut header = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            // SAFETY: `pixels` holds width * height 32-bit pixels.
            let lines = unsafe {
                let dc = CreateCompatibleDC(None);
                let lines = GetDIBits(
                    dc,
                    info.hbmColor,
                    0,
                    height as u32,
                    Some(pixels.as_mut_ptr().cast()),
                    &mut header,
                    DIB_RGB_COLORS,
                );
                let _ = DeleteDC(dc);
                lines
            };
            if lines != height {
                return Err(windows::core::Error::from_thread());
            }
            // Icons carry straight alpha; Direct2D wants it premultiplied.
            for pixel in pixels.as_chunks_mut::<4>().0 {
                let alpha = u32::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
            let properties = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            };
            // SAFETY: `pixels` is a width * height bitmap with the given pitch.
            unsafe {
                target.CreateBitmap(
                    D2D_SIZE_U {
                        width: width as u32,
                        height: height as u32,
                    },
                    Some(pixels.as_ptr().cast()),
                    (width * 4) as u32,
                    &properties,
                )
            }
        })();
        // SAFETY: the bitmaps came from GetIconInfo and are deleted once.
        unsafe {
            let _ = DeleteObject(info.hbmColor.into());
            let _ = DeleteObject(info.hbmMask.into());
        }
        result
    }

    /// The width of `text` set on one line.
    pub fn text_width(&self, text: &str, format: &IDWriteTextFormat) -> f32 {
        self.layout(text, format, 100_000.0, 100.0)
            .and_then(|layout| {
                let mut metrics = DWRITE_TEXT_METRICS::default();
                // SAFETY: `metrics` is a valid out parameter.
                unsafe { layout.GetMetrics(&mut metrics) }
                    .map(|_| metrics.widthIncludingTrailingWhitespace)
            })
            .unwrap_or(0.0)
    }

    /// A Direct2D render target that draws into a GDI device context.
    pub fn dc_render_target(&self) -> Result<ID2D1DCRenderTarget> {
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        // SAFETY: plain resource creation.
        let target = unsafe { self.factory.CreateDCRenderTarget(&properties)? };
        // Text is drawn onto a transparent backdrop, where ClearType cannot be used.
        // SAFETY: as above.
        unsafe { target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE) };
        Ok(target)
    }

    /// Renders Spectacle's glasses glyph as a `size` pixel square icon.
    pub fn tray_icon(&self, size: i32, color: D2D1_COLOR_F) -> Result<HICON> {
        let surface = DibSurface::new(size, size)?;
        let target = self.dc_render_target()?;
        let bounds = RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        // SAFETY: the surface's device context stays alive while drawing.
        unsafe {
            target.BindDC(surface.dc, &bounds)?;
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            self.draw_glasses(&target, size as f32, color)?;
            target.EndDraw(None, None)?;
        }
        surface.to_icon()
    }

    /// Draws the glasses of the macOS status item, traced from its artwork on
    /// a 40 unit grid and scaled to fill `size` pixels.
    fn draw_glasses(
        &self,
        target: &ID2D1RenderTarget,
        size: f32,
        color: D2D1_COLOR_F,
    ) -> Result<()> {
        // With its strokes, the artwork spans x 1.95..38.05 and y 8.0..30.9 on
        // its grid. It is centred with a margin of a pixel per 16 pixels.
        let margin = (size / 16.0).floor().max(1.0);
        let scale = (size - 2.0 * margin) / 36.1;
        let offset_x = (size - 36.1 * scale) / 2.0 - 1.95 * scale;
        let offset_y = (size - 22.9 * scale) / 2.0 - 8.0 * scale;
        let point = |x: f32, y: f32| Vector2 {
            X: offset_x + x * scale,
            Y: offset_y + y * scale,
        };
        // SAFETY: plain resource creation and drawing on a target in BeginDraw.
        unsafe {
            let brush = target.CreateSolidColorBrush(&color, None)?;
            let outline = self.factory.CreatePathGeometry()?;
            let sink = outline.Open()?;
            for mirror in [false, true] {
                let p = |x: f32, y: f32| {
                    if mirror {
                        point(40.0 - x, y)
                    } else {
                        point(x, y)
                    }
                };
                // Lens: a flat top with a rounded bottom.
                sink.BeginFigure(p(3.2, 18.7), D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddLine(p(16.9, 18.7));
                sink.AddLine(p(16.9, 22.8));
                sink.AddBezier(
                    &windows::Win32::Graphics::Direct2D::Common::D2D1_BEZIER_SEGMENT {
                        point1: p(16.9, 27.0),
                        point2: p(14.4, 29.6),
                        point3: p(10.05, 29.6),
                    },
                );
                sink.AddBezier(
                    &windows::Win32::Graphics::Direct2D::Common::D2D1_BEZIER_SEGMENT {
                        point1: p(5.7, 29.6),
                        point2: p(3.2, 27.0),
                        point3: p(3.2, 22.8),
                    },
                );
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                // Temple, rising from the top of the lens to a small hook.
                sink.BeginFigure(p(4.0, 17.6), D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddLine(p(10.9, 10.7));
                sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                    point1: p(12.6, 9.2),
                    point2: p(13.3, 11.0),
                });
                sink.AddLine(p(13.5, 12.4));
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
            }
            // Bridge between the lenses.
            sink.BeginFigure(point(16.9, 18.7), D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddLine(point(23.1, 18.7));
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            sink.Close()?;
            target.DrawGeometry(&outline, &brush, 2.5 * scale, &self.round_stroke);

            let bridge = self.factory.CreatePathGeometry()?;
            let sink = bridge.Open()?;
            sink.BeginFigure(point(16.5, 18.0), D2D1_FIGURE_BEGIN_FILLED);
            sink.AddLine(point(23.5, 18.0));
            sink.AddLine(point(23.5, 21.3));
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                point1: point(20.0, 19.3),
                point2: point(16.5, 21.3),
            });
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            target.FillGeometry(&bridge, &brush, None);
        }
        Ok(())
    }
}

/// Draws one line of text centred vertically within `rect`.
pub fn draw_text(
    target: &ID2D1RenderTarget,
    layout: &IDWriteTextLayout,
    x: f32,
    y: f32,
    color: D2D1_COLOR_F,
) {
    // SAFETY: drawing on a target between BeginDraw and EndDraw.
    unsafe {
        if let Ok(brush) = target.CreateSolidColorBrush(&color, None) {
            target.DrawTextLayout(
                Vector2 { X: x, Y: y },
                layout,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
    }
}

pub fn rect_f(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

fn first_installed_family(dwrite: &IDWriteFactory, families: &[&str]) -> HSTRING {
    let mut collection = None;
    // SAFETY: `collection` is a valid out parameter.
    if unsafe { dwrite.GetSystemFontCollection(&mut collection, false) }.is_ok()
        && let Some(collection) = collection
    {
        for family in families {
            let mut index = 0;
            let mut exists = false.into();
            // SAFETY: valid string and out parameters.
            if unsafe {
                collection.FindFamilyName(&HSTRING::from(*family), &mut index, &mut exists)
            }
            .is_ok()
                && exists.as_bool()
            {
                return HSTRING::from(*family);
            }
        }
    }
    HSTRING::from(*families.last().unwrap_or(&"Segoe UI"))
}

/// A 32-bit top-down DIB selected into a memory device context.
pub struct DibSurface {
    pub dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    pub width: i32,
    pub height: i32,
}

impl DibSurface {
    pub fn new(width: i32, height: i32) -> Result<DibSurface> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width.max(1),
                biHeight: -height.max(1),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        // SAFETY: creates a memory DC and DIB section that this value owns.
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err(windows::core::Error::from_thread());
            }
            let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            {
                Ok(bitmap) => bitmap,
                Err(error) => {
                    let _ = DeleteDC(dc);
                    return Err(error);
                }
            };
            let previous = SelectObject(dc, bitmap.into());
            Ok(DibSurface {
                dc,
                bitmap,
                previous,
                bits: bits.cast(),
                width: width.max(1),
                height: height.max(1),
            })
        }
    }

    pub fn pixels(&self) -> &[u8] {
        // SAFETY: the DIB section holds width * height 32-bit pixels.
        unsafe { std::slice::from_raw_parts(self.bits, (self.width * self.height * 4) as usize) }
    }

    /// Converts the premultiplied pixels to an icon, whose colour bitmap
    /// Windows expects with straight alpha.
    fn to_icon(&self) -> Result<HICON> {
        let surface = DibSurface::new(self.width, self.height)?;
        // SAFETY: both surfaces hold width * height 32-bit pixels.
        let destination = unsafe {
            std::slice::from_raw_parts_mut(surface.bits, (self.width * self.height * 4) as usize)
        };
        for (source, destination) in self
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(destination.as_chunks_mut::<4>().0)
        {
            let alpha = u32::from(source[3]);
            for channel in 0..3 {
                destination[channel] = (u32::from(source[channel]) * 255 + alpha / 2)
                    .checked_div(alpha)
                    .map_or(0, |value| value.min(255) as u8);
            }
            destination[3] = source[3];
        }
        // SAFETY: the mask is a monochrome bitmap freed once the icon exists.
        unsafe {
            let mask = CreateBitmap(self.width, self.height, 1, 1, None);
            let info = ICONINFO {
                fIcon: true.into(),
                xHotspot: 0,
                yHotspot: 0,
                hbmMask: mask,
                hbmColor: surface.bitmap,
            };
            let icon = CreateIconIndirect(&info);
            let _ = DeleteObject(mask.into());
            icon
        }
    }
}

impl Drop for DibSurface {
    fn drop(&mut self) {
        // SAFETY: the objects were created by DibSurface::new and are released once.
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders the tray glyph and checks that it is drawn, symmetric and
    /// within the icon.
    #[test]
    fn tray_glyph_is_symmetric_and_inside_the_icon() {
        let graphics = Graphics::new().unwrap();
        for size in [16, 20, 24, 32] {
            let surface = DibSurface::new(size, size).unwrap();
            let target = graphics.dc_render_target().unwrap();
            let bounds = RECT {
                left: 0,
                top: 0,
                right: size,
                bottom: size,
            };
            let white = D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };
            unsafe {
                target.BindDC(surface.dc, &bounds).unwrap();
                target.BeginDraw();
                target.Clear(Some(&D2D1_COLOR_F::default()));
                graphics.draw_glasses(&target, size as f32, white).unwrap();
                target.EndDraw(None, None).unwrap();
            }
            let alpha =
                |x: i32, y: i32| i32::from(surface.pixels()[((y * size + x) * 4 + 3) as usize]);
            let mut total = 0;
            let mut asymmetry = 0;
            for y in 0..size {
                for x in 0..size {
                    total += alpha(x, y);
                    asymmetry += (alpha(x, y) - alpha(size - 1 - x, y)).abs();
                }
                assert_eq!(
                    alpha(0, y) + alpha(size - 1, y),
                    0,
                    "glyph touches the edge at {size}px"
                );
            }
            assert!(total > size * size * 255 / 8, "glyph too faint at {size}px");
            assert!(asymmetry * 20 < total, "glyph asymmetric at {size}px");
        }
    }
}

#[cfg(test)]
mod preview {
    use super::*;

    #[test]
    #[ignore = "prints the tray glyph for visual inspection"]
    fn print_tray_glyph() {
        let graphics = Graphics::new().unwrap();
        for size in [16, 24, 40] {
            let surface = DibSurface::new(size, size).unwrap();
            let target = graphics.dc_render_target().unwrap();
            let bounds = RECT {
                left: 0,
                top: 0,
                right: size,
                bottom: size,
            };
            let white = D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };
            unsafe {
                target.BindDC(surface.dc, &bounds).unwrap();
                target.BeginDraw();
                target.Clear(Some(&D2D1_COLOR_F::default()));
                graphics.draw_glasses(&target, size as f32, white).unwrap();
                target.EndDraw(None, None).unwrap();
            }
            let ramp: Vec<char> = " .:-=+*#%@".chars().collect();
            for y in 0..size {
                let line: String = (0..size)
                    .map(|x| {
                        ramp[(surface.pixels()[((y * size + x) * 4 + 3) as usize] as usize * 10
                            / 256)
                            .min(9)]
                    })
                    .collect();
                println!("|{line}|");
            }
        }
    }
}
