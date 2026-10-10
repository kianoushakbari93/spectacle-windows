//! Moves a window to a calculated rectangle, working around windows that
//! cannot take the exact size asked for.
//!
//! This is the macOS version's chain of standard, quantized and best effort
//! window movers. All rectangles are in Windows screen coordinates.

use crate::geometry::Rect;

/// A window Spectacle can position.
pub trait WindowElement: Clone {
    /// The window's visible bounds, or `None` once it can no longer be read.
    fn rect(&self) -> Option<Rect>;
    /// Asks the window to take on visible bounds of `rect`; returns whether
    /// the request was delivered.
    fn set_rect(&self, rect: Rect) -> bool;
    /// Whether the window can be resized, rather than only moved.
    fn is_resizable(&self) -> bool {
        true
    }
    /// Whether the window still exists.
    fn is_alive(&self) -> bool;
    /// Whether the window is maximized.
    fn is_maximized(&self) -> bool {
        false
    }
    /// Maximizes the window on the display it is on.
    fn maximize(&self) -> bool {
        false
    }
}

/// Moves `window` to `rect` and keeps it within `visible_frame_of_screen`.
/// Returns whether the window accepted the move.
pub fn move_window<W: WindowElement>(
    window: &W,
    rect: Rect,
    visible_frame_of_screen: Rect,
) -> bool {
    // Standard mover: ask for the exact rectangle.
    if window.rect().is_none() || !window.set_rect(rect) {
        return false;
    }
    move_window_quantized(window, rect);
    move_window_best_effort(window, visible_frame_of_screen);
    true
}

/// Windows whose size snaps to a grid, such as terminals, may end up larger
/// than asked. Shrink the request step by step until such a window fits, then
/// centre it within the intended rectangle. Fixed size windows are centred
/// straight away.
fn move_window_quantized<W: WindowElement>(window: &W, rect: Rect) {
    let Some(mut moved) = window.rect() else {
        return;
    };
    if moved == rect {
        return;
    }
    let mut adjusted = rect;
    while window.is_resizable() && (moved.width > rect.width || moved.height > rect.height) {
        if moved.width > rect.width {
            adjusted.width -= 2.0;
        }
        if moved.height > rect.height {
            adjusted.height -= 2.0;
        }
        if adjusted.width < rect.width * 0.85 || adjusted.height < rect.height * 0.85 {
            break;
        }
        window.set_rect(adjusted);
        match window.rect() {
            Some(rect) => moved = rect,
            None => return,
        }
    }
    adjusted.x += ((rect.width - moved.width) / 2.0).floor();
    adjusted.y += ((rect.height - moved.height) / 2.0).floor();
    window.set_rect(adjusted);
}

/// Pulls a window that ended up partly off screen back within the visible
/// frame of the screen.
fn move_window_best_effort<W: WindowElement>(window: &W, visible_frame_of_screen: Rect) {
    let Some(moved) = window.rect() else {
        return;
    };
    let mut clamped = moved;
    if clamped.x < visible_frame_of_screen.x {
        clamped.x = visible_frame_of_screen.x;
    } else if clamped.max_x() > visible_frame_of_screen.max_x() {
        clamped.x = visible_frame_of_screen.max_x() - clamped.width;
    }
    if clamped.y < visible_frame_of_screen.y {
        clamped.y = visible_frame_of_screen.y;
    } else if clamped.max_y() > visible_frame_of_screen.max_y() {
        clamped.y = visible_frame_of_screen.max_y() - clamped.height;
    }
    if clamped != moved {
        window.set_rect(clamped);
    }
}

#[cfg(test)]
pub mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    /// A window that honours requests within its size constraints.
    #[derive(Clone)]
    pub struct FakeWindow {
        pub rect: Rc<RefCell<Option<Rect>>>,
        pub minimum_width: f64,
        pub minimum_height: f64,
        pub grid: f64,
        pub resizable: bool,
        pub requests: Rc<RefCell<usize>>,
        pub maximized: Rc<RefCell<bool>>,
    }

    impl FakeWindow {
        pub fn new(rect: Rect) -> Self {
            Self {
                rect: Rc::new(RefCell::new(Some(rect))),
                minimum_width: 0.0,
                minimum_height: 0.0,
                grid: 1.0,
                resizable: true,
                requests: Rc::new(RefCell::new(0)),
                maximized: Rc::new(RefCell::new(false)),
            }
        }

        pub fn current(&self) -> Rect {
            self.rect.borrow().expect("window is closed")
        }

        pub fn close(&self) {
            *self.rect.borrow_mut() = None;
        }
    }

    impl WindowElement for FakeWindow {
        fn rect(&self) -> Option<Rect> {
            *self.rect.borrow()
        }

        fn set_rect(&self, rect: Rect) -> bool {
            *self.requests.borrow_mut() += 1;
            *self.maximized.borrow_mut() = false;
            let mut current = self.rect.borrow_mut();
            let Some(previous) = *current else {
                return false;
            };
            let mut applied = rect;
            if self.resizable {
                applied.width = (rect.width.max(self.minimum_width) / self.grid).ceil() * self.grid;
                applied.height =
                    (rect.height.max(self.minimum_height) / self.grid).ceil() * self.grid;
            } else {
                applied.width = previous.width;
                applied.height = previous.height;
            }
            *current = Some(applied);
            true
        }

        fn is_resizable(&self) -> bool {
            self.resizable
        }

        fn is_alive(&self) -> bool {
            self.rect.borrow().is_some()
        }

        fn is_maximized(&self) -> bool {
            *self.maximized.borrow()
        }

        fn maximize(&self) -> bool {
            *self.maximized.borrow_mut() = true;
            true
        }
    }

    const SCREEN: Rect = Rect::new(0.0, 0.0, 1920.0, 1040.0);

    #[test]
    fn moves_an_unconstrained_window_exactly() {
        let window = FakeWindow::new(Rect::new(100.0, 100.0, 500.0, 400.0));
        let target = Rect::new(0.0, 0.0, 960.0, 1040.0);
        assert!(move_window(&window, target, SCREEN));
        assert_eq!(window.current(), target);
        assert_eq!(*window.requests.borrow(), 1);
    }

    #[test]
    fn centres_a_window_that_cannot_shrink_and_keeps_it_on_screen() {
        let mut window = FakeWindow::new(Rect::new(100.0, 100.0, 500.0, 400.0));
        window.minimum_width = 1100.0;
        let target = Rect::new(960.0, 0.0, 960.0, 1040.0);
        assert!(move_window(&window, target, SCREEN));
        let moved = window.current();
        assert_eq!(moved.width, 1100.0);
        assert!(moved.max_x() <= SCREEN.max_x());
        assert!(moved.x >= SCREEN.x);
    }

    #[test]
    fn shrinks_a_grid_snapped_window_until_it_fits() {
        let mut window = FakeWindow::new(Rect::new(100.0, 100.0, 500.0, 400.0));
        window.grid = 7.0;
        let target = Rect::new(0.0, 0.0, 960.0, 520.0);
        assert!(move_window(&window, target, SCREEN));
        let moved = window.current();
        assert!(moved.width <= target.width && moved.height <= target.height);
        assert!((moved.mid_x() - target.mid_x()).abs() <= 1.0);
        assert!((moved.mid_y() - target.mid_y()).abs() <= 1.0);
    }

    #[test]
    fn centres_a_fixed_size_window_without_resizing_it() {
        let mut window = FakeWindow::new(Rect::new(100.0, 100.0, 500.0, 400.0));
        window.resizable = false;
        assert!(move_window(
            &window,
            Rect::new(0.0, 0.0, 960.0, 1040.0),
            SCREEN
        ));
        assert_eq!(window.current(), Rect::new(230.0, 320.0, 500.0, 400.0));
        assert_eq!(*window.requests.borrow(), 2);
    }

    #[test]
    fn fails_for_a_closed_window() {
        let window = FakeWindow::new(Rect::new(100.0, 100.0, 500.0, 400.0));
        window.close();
        assert!(!move_window(
            &window,
            Rect::new(0.0, 0.0, 960.0, 1040.0),
            SCREEN
        ));
    }
}
