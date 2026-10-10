//! Performs window actions on a window and keeps each application's undo and
//! redo history.

use std::collections::HashMap;

use crate::action::WindowAction;
use crate::calculations::calculate_window_rect;
use crate::geometry::Rect;
use crate::history::History;
use crate::movers::{WindowElement, move_window};
use crate::screens::{self, Screen};

struct HistoryItem<W> {
    window: W,
    rect: Rect,
    /// Whether the window was maximized, which undo restores.
    maximized: bool,
}

pub struct WindowPositionManager<W: WindowElement> {
    application_histories: HashMap<String, History<HistoryItem<W>>>,
}

impl<W: WindowElement> Default for WindowPositionManager<W> {
    fn default() -> Self {
        Self {
            application_histories: HashMap::new(),
        }
    }
}

impl<W: WindowElement> WindowPositionManager<W> {
    /// Performs `action` on `window`, which belongs to `application`.
    /// Returns `false` when nothing could be done, so the caller can let the
    /// user know.
    pub fn perform(
        &mut self,
        action: WindowAction,
        window: Option<W>,
        application: &str,
        screens: &[Screen],
    ) -> bool {
        match action {
            WindowAction::Undo => {
                self.move_with_history(application, screens, |history| history.previous())
            }
            WindowAction::Redo => {
                self.move_with_history(application, screens, |history| history.next())
            }
            _ => match window {
                Some(window) => self.move_window(action, window, application, screens),
                None => false,
            },
        }
    }

    fn move_window(
        &mut self,
        action: WindowAction,
        window: W,
        application: &str,
        screens: &[Screen],
    ) -> bool {
        let Some(window_rect) = window.rect() else {
            return false;
        };
        let Some(detection) = screens::detect(action, window_rect, screens) else {
            return false;
        };
        let Some(calculated) = calculate_window_rect(
            action,
            window_rect.flipped(),
            detection.source.visible_frame.flipped(),
            detection.destination.visible_frame.flipped(),
        )
        .map(|rect| rect.flipped()) else {
            return false;
        };
        if calculated == window_rect {
            return false;
        }
        let history = self
            .application_histories
            .entry(application.to_owned())
            .or_default();
        if history.is_empty() {
            history.add(HistoryItem {
                window: window.clone(),
                rect: window_rect,
                maximized: window.is_maximized(),
            });
        }
        history.add(HistoryItem {
            window: window.clone(),
            rect: calculated,
            maximized: false,
        });
        move_window(&window, calculated, detection.destination.visible_frame)
    }

    fn move_with_history(
        &mut self,
        application: &str,
        screens: &[Screen],
        step: impl FnOnce(&mut History<HistoryItem<W>>) -> Option<&HistoryItem<W>>,
    ) -> bool {
        let Some(history) = self.application_histories.get_mut(application) else {
            return false;
        };
        history.retain(|item| item.window.is_alive());
        let Some(item) = step(history) else {
            return false;
        };
        let Some(screen) = screens::screen_containing_rect(item.rect, screens) else {
            return false;
        };
        // A maximized window is put back on its display, then maximized again.
        let moved = move_window(&item.window, item.rect, screen.visible_frame);
        if item.maximized {
            item.window.maximize()
        } else {
            moved
        }
    }

    /// Forgets the history of windows that no longer exist.
    pub fn prune(&mut self) {
        self.application_histories.retain(|_, history| {
            history.retain(|item| item.window.is_alive());
            !history.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movers::tests::FakeWindow;

    const APP: &str = "c:\\program files\\app\\app.exe";

    fn screens() -> Vec<Screen> {
        vec![Screen {
            frame: Rect::new(0.0, 0.0, 1920.0, 1080.0),
            visible_frame: Rect::new(0.0, 0.0, 1920.0, 1032.0),
        }]
    }

    fn dual_screens() -> Vec<Screen> {
        let mut screens = screens();
        screens.push(Screen {
            frame: Rect::new(1920.0, 0.0, 2560.0, 1440.0),
            visible_frame: Rect::new(1920.0, 0.0, 2560.0, 1392.0),
        });
        screens
    }

    #[test]
    fn moves_a_window_to_the_left_half_in_windows_coordinates() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        assert!(manager.perform(
            WindowAction::LeftHalf,
            Some(window.clone()),
            APP,
            &screens()
        ));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 960.0, 1032.0));
    }

    #[test]
    fn top_half_is_at_the_top_of_the_screen() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        assert!(manager.perform(WindowAction::TopHalf, Some(window.clone()), APP, &screens()));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 1920.0, 516.0));
        assert!(manager.perform(
            WindowAction::BottomHalf,
            Some(window.clone()),
            APP,
            &screens()
        ));
        assert_eq!(window.current(), Rect::new(0.0, 516.0, 1920.0, 516.0));
    }

    #[test]
    fn upper_left_and_lower_right_are_the_right_corners() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        assert!(manager.perform(
            WindowAction::UpperLeft,
            Some(window.clone()),
            APP,
            &screens()
        ));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 960.0, 516.0));
        assert!(manager.perform(
            WindowAction::LowerRight,
            Some(window.clone()),
            APP,
            &screens()
        ));
        assert_eq!(window.current(), Rect::new(960.0, 516.0, 960.0, 516.0));
    }

    #[test]
    fn repeating_an_action_that_changes_nothing_fails() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        assert!(manager.perform(
            WindowAction::Fullscreen,
            Some(window.clone()),
            APP,
            &screens()
        ));
        assert!(!manager.perform(WindowAction::Fullscreen, Some(window), APP, &screens()));
    }

    #[test]
    fn undoes_and_redoes_window_actions() {
        let mut manager = WindowPositionManager::default();
        let original = Rect::new(300.0, 200.0, 800.0, 600.0);
        let window = FakeWindow::new(original);
        manager.perform(
            WindowAction::LeftHalf,
            Some(window.clone()),
            APP,
            &screens(),
        );
        manager.perform(
            WindowAction::Fullscreen,
            Some(window.clone()),
            APP,
            &screens(),
        );
        assert!(manager.perform(WindowAction::Undo, None, APP, &screens()));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 960.0, 1032.0));
        assert!(manager.perform(WindowAction::Undo, None, APP, &screens()));
        assert_eq!(window.current(), original);
        assert!(!manager.perform(WindowAction::Undo, None, APP, &screens()));
        assert!(manager.perform(WindowAction::Redo, None, APP, &screens()));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 960.0, 1032.0));
        assert!(manager.perform(WindowAction::Redo, None, APP, &screens()));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 1920.0, 1032.0));
        assert!(!manager.perform(WindowAction::Redo, None, APP, &screens()));
    }

    #[test]
    fn undo_maximizes_a_window_that_was_maximized() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(0.0, 0.0, 1920.0, 1032.0));
        *window.maximized.borrow_mut() = true;
        assert!(manager.perform(
            WindowAction::LeftHalf,
            Some(window.clone()),
            APP,
            &screens()
        ));
        assert!(!window.is_maximized());
        assert!(manager.perform(WindowAction::Undo, None, APP, &screens()));
        assert!(window.is_maximized());
        assert!(manager.perform(WindowAction::Redo, None, APP, &screens()));
        assert!(!window.is_maximized());
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 960.0, 1032.0));
    }

    #[test]
    fn histories_are_kept_per_application() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        manager.perform(WindowAction::LeftHalf, Some(window), APP, &screens());
        assert!(!manager.perform(WindowAction::Undo, None, "c:\\other.exe", &screens()));
    }

    #[test]
    fn undo_skips_closed_windows() {
        let mut manager = WindowPositionManager::default();
        let closed = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        manager.perform(
            WindowAction::LeftHalf,
            Some(closed.clone()),
            APP,
            &screens(),
        );
        closed.close();
        assert!(!manager.perform(WindowAction::Undo, None, APP, &screens()));
        manager.prune();
        assert!(manager.application_histories.is_empty());
    }

    #[test]
    fn moves_a_window_to_the_next_display() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        assert!(manager.perform(
            WindowAction::NextDisplay,
            Some(window.clone()),
            APP,
            &dual_screens()
        ));
        assert_eq!(
            window.current(),
            Rect::new(1920.0 + 880.0, 396.0, 800.0, 600.0)
        );
        assert!(manager.perform(
            WindowAction::PreviousDisplay,
            Some(window.clone()),
            APP,
            &dual_screens()
        ));
        assert_eq!(window.current(), Rect::new(560.0, 216.0, 800.0, 600.0));
    }

    #[test]
    fn a_window_too_large_for_the_next_display_fills_it() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(1920.0, 0.0, 2560.0, 1392.0));
        assert!(manager.perform(
            WindowAction::NextDisplay,
            Some(window.clone()),
            APP,
            &dual_screens()
        ));
        assert_eq!(window.current(), Rect::new(0.0, 0.0, 1920.0, 1032.0));
    }

    #[test]
    fn display_actions_fail_with_a_single_display() {
        let mut manager = WindowPositionManager::default();
        let window = FakeWindow::new(Rect::new(300.0, 200.0, 800.0, 600.0));
        assert!(!manager.perform(WindowAction::NextDisplay, Some(window), APP, &screens()));
    }

    #[test]
    fn no_window_fails() {
        let mut manager = WindowPositionManager::<FakeWindow>::default();
        assert!(!manager.perform(WindowAction::Center, None, APP, &screens()));
    }
}
