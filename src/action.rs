//! The window actions Spectacle performs, one per keyboard shortcut.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowAction {
    Center,
    Fullscreen,
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    UpperLeft,
    LowerLeft,
    UpperRight,
    LowerRight,
    NextDisplay,
    PreviousDisplay,
    NextThird,
    PreviousThird,
    Larger,
    Smaller,
    Undo,
    Redo,
}

impl WindowAction {
    pub const ALL: [WindowAction; 18] = [
        WindowAction::Center,
        WindowAction::Fullscreen,
        WindowAction::LeftHalf,
        WindowAction::RightHalf,
        WindowAction::TopHalf,
        WindowAction::BottomHalf,
        WindowAction::UpperLeft,
        WindowAction::LowerLeft,
        WindowAction::UpperRight,
        WindowAction::LowerRight,
        WindowAction::NextDisplay,
        WindowAction::PreviousDisplay,
        WindowAction::NextThird,
        WindowAction::PreviousThird,
        WindowAction::Larger,
        WindowAction::Smaller,
        WindowAction::Undo,
        WindowAction::Redo,
    ];

    /// The name a shortcut for this action is stored under in Shortcuts.json.
    /// These match the names the macOS version uses.
    pub const fn shortcut_name(self) -> &'static str {
        match self {
            WindowAction::Center => "MoveToCenter",
            WindowAction::Fullscreen => "MoveToFullscreen",
            WindowAction::LeftHalf => "MoveToLeftHalf",
            WindowAction::RightHalf => "MoveToRightHalf",
            WindowAction::TopHalf => "MoveToTopHalf",
            WindowAction::BottomHalf => "MoveToBottomHalf",
            WindowAction::UpperLeft => "MoveToUpperLeft",
            WindowAction::LowerLeft => "MoveToLowerLeft",
            WindowAction::UpperRight => "MoveToUpperRight",
            WindowAction::LowerRight => "MoveToLowerRight",
            WindowAction::NextDisplay => "MoveToNextDisplay",
            WindowAction::PreviousDisplay => "MoveToPreviousDisplay",
            WindowAction::NextThird => "MoveToNextThird",
            WindowAction::PreviousThird => "MoveToPreviousThird",
            WindowAction::Larger => "MakeLarger",
            WindowAction::Smaller => "MakeSmaller",
            WindowAction::Undo => "UndoLastMove",
            WindowAction::Redo => "RedoLastMove",
        }
    }

    #[cfg(test)]
    pub fn from_shortcut_name(name: &str) -> Option<WindowAction> {
        WindowAction::ALL
            .into_iter()
            .find(|action| action.shortcut_name() == name)
    }

    /// A stable index in `ALL`, also used as the hotkey identifier.
    pub fn index(self) -> usize {
        WindowAction::ALL
            .iter()
            .position(|action| *action == self)
            .unwrap_or_default()
    }

    pub fn is_moving_to_display(self) -> bool {
        matches!(
            self,
            WindowAction::NextDisplay | WindowAction::PreviousDisplay
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_names_round_trip() {
        for action in WindowAction::ALL {
            assert_eq!(
                WindowAction::from_shortcut_name(action.shortcut_name()),
                Some(action)
            );
        }
        assert_eq!(WindowAction::from_shortcut_name("MoveToNowhere"), None);
    }

    #[test]
    fn indices_are_positions_in_all() {
        for (index, action) in WindowAction::ALL.into_iter().enumerate() {
            assert_eq!(action.index(), index);
        }
    }
}
