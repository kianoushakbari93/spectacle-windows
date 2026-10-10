//! Display enumeration and detection of the displays a window action moves a
//! window from and to.

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::core::BOOL;

use crate::action::WindowAction;
use crate::geometry::Rect;

/// A display in Windows screen coordinates (y grows downwards).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    /// The full bounds of the display.
    pub frame: Rect,
    /// The bounds of the display less the taskbar and docked app bars.
    pub visible_frame: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenDetectionResult {
    pub source: Screen,
    pub destination: Screen,
}

/// Lists the attached displays.
pub fn screens() -> Vec<Screen> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        // SAFETY: `data` is the address of the vector passed to
        // EnumDisplayMonitors below, which outlives the enumeration.
        let screens = unsafe { &mut *(data.0 as *mut Vec<Screen>) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` is a properly sized MONITORINFO.
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            screens.push(Screen {
                frame: Rect::from_win32(&info.rcMonitor),
                visible_frame: Rect::from_win32(&info.rcWork),
            });
        }
        BOOL::from(true)
    }
    let mut screens: Vec<Screen> = Vec::new();
    // SAFETY: the callback only dereferences `screens`, which lives across the call.
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut screens as *mut _ as isize),
        )
    };
    screens
}

/// Finds the display a window is on and, for the display actions, the
/// display it should move to.
pub fn detect(
    action: WindowAction,
    window_rect: Rect,
    screens: &[Screen],
) -> Option<ScreenDetectionResult> {
    let screens = screens_in_consistent_order(screens);
    let source = screen_containing_rect(window_rect, &screens)?;
    let destination = if action.is_moving_to_display() {
        next_or_previous_screen(source, action, &screens)?
    } else {
        source
    };
    Some(ScreenDetectionResult {
        source,
        destination,
    })
}

/// The display holding a rectangle: the one containing it entirely, else the
/// one holding most of it, else the nearest one.
pub fn screen_containing_rect(rect: Rect, screens: &[Screen]) -> Option<Screen> {
    if let Some(screen) = screens
        .iter()
        .find(|screen| screen.frame.contains_rect(&rect))
    {
        return Some(*screen);
    }
    let largest_overlap = screens
        .iter()
        .filter_map(|screen| {
            let overlap = screen.frame.intersection(&rect)?.area() / rect.area();
            (overlap > 0.0).then_some((overlap, *screen))
        })
        .max_by(|(a, _), (b, _)| a.total_cmp(b));
    if let Some((_, screen)) = largest_overlap {
        return Some(screen);
    }
    screens
        .iter()
        .min_by(|a, b| distance_squared(a.frame, rect).total_cmp(&distance_squared(b.frame, rect)))
        .copied()
}

fn distance_squared(frame: Rect, rect: Rect) -> f64 {
    let dx = frame.mid_x() - rect.mid_x();
    let dy = frame.mid_y() - rect.mid_y();
    dx * dx + dy * dy
}

/// Orders displays left to right, then top to bottom, so that "next display"
/// moves rightwards (or downwards) through the arrangement and wraps around.
fn screens_in_consistent_order(screens: &[Screen]) -> Vec<Screen> {
    let mut ordered = screens.to_vec();
    ordered.sort_by(|a, b| {
        a.frame
            .x
            .total_cmp(&b.frame.x)
            .then(a.frame.y.total_cmp(&b.frame.y))
    });
    ordered
}

fn next_or_previous_screen(
    source: Screen,
    action: WindowAction,
    screens: &[Screen],
) -> Option<Screen> {
    if screens.len() <= 1 {
        return None;
    }
    let index = screens
        .iter()
        .position(|screen| screen.frame == source.frame)?;
    let count = screens.len();
    let index = match action {
        WindowAction::NextDisplay => (index + 1) % count,
        WindowAction::PreviousDisplay => (index + count - 1) % count,
        _ => index,
    };
    Some(screens[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(x: f64, y: f64, width: f64, height: f64) -> Screen {
        Screen {
            frame: Rect::new(x, y, width, height),
            visible_frame: Rect::new(x, y, width, height - 48.0),
        }
    }

    #[test]
    fn finds_the_screen_containing_a_window() {
        let left = screen(-1920.0, 0.0, 1920.0, 1080.0);
        let primary = screen(0.0, 0.0, 2560.0, 1440.0);
        let result = detect(
            WindowAction::LeftHalf,
            Rect::new(100.0, 100.0, 800.0, 600.0),
            &[primary, left],
        )
        .unwrap();
        assert_eq!(result.source, primary);
        assert_eq!(result.destination, primary);
    }

    #[test]
    fn prefers_the_screen_holding_most_of_a_window() {
        let left = screen(-1920.0, 0.0, 1920.0, 1080.0);
        let primary = screen(0.0, 0.0, 2560.0, 1440.0);
        let straddling = Rect::new(-700.0, 100.0, 1000.0, 600.0);
        assert_eq!(
            screen_containing_rect(straddling, &[primary, left]),
            Some(left)
        );
    }

    #[test]
    fn falls_back_to_the_nearest_screen() {
        let left = screen(-1920.0, 0.0, 1920.0, 1080.0);
        let primary = screen(0.0, 0.0, 2560.0, 1440.0);
        let offscreen = Rect::new(5000.0, 100.0, 300.0, 300.0);
        assert_eq!(
            screen_containing_rect(offscreen, &[left, primary]),
            Some(primary)
        );
    }

    #[test]
    fn next_and_previous_display_wrap_around_left_to_right() {
        let left = screen(-1920.0, 0.0, 1920.0, 1080.0);
        let middle = screen(0.0, 0.0, 2560.0, 1440.0);
        let right = screen(2560.0, 0.0, 1920.0, 1080.0);
        let screens = [middle, right, left];
        let window = Rect::new(2600.0, 100.0, 800.0, 600.0);
        assert_eq!(
            detect(WindowAction::NextDisplay, window, &screens)
                .unwrap()
                .destination,
            left
        );
        assert_eq!(
            detect(WindowAction::PreviousDisplay, window, &screens)
                .unwrap()
                .destination,
            middle
        );
    }

    #[test]
    fn display_actions_need_a_second_display() {
        let only = screen(0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(
            detect(
                WindowAction::NextDisplay,
                Rect::new(0.0, 0.0, 10.0, 10.0),
                &[only]
            ),
            None
        );
    }

    #[test]
    fn no_screens_detects_nothing() {
        assert_eq!(
            detect(WindowAction::Center, Rect::new(0.0, 0.0, 10.0, 10.0), &[]),
            None
        );
    }
}
