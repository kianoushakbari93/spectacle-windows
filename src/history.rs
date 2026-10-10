//! Undo and redo history of window actions.
//!
//! Behaves like the macOS version's linked list: adding an item after undoing
//! discards the items that could have been redone, and only the most recent
//! fifty items are kept.

const MAXIMUM_SIZE: usize = 50;

#[derive(Debug)]
pub struct History<T> {
    items: Vec<T>,
    current: usize,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            current: 0,
        }
    }
}

impl<T> History<T> {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn add(&mut self, item: T) {
        if !self.items.is_empty() {
            self.items.truncate(self.current + 1);
        }
        self.items.push(item);
        if self.items.len() > MAXIMUM_SIZE {
            self.items.remove(0);
        }
        self.current = self.items.len() - 1;
    }

    /// Steps back and returns the item to restore, if there is one.
    pub fn previous(&mut self) -> Option<&T> {
        if self.items.is_empty() || self.current == 0 {
            return None;
        }
        self.current -= 1;
        self.items.get(self.current)
    }

    /// Steps forward and returns the item to restore, if there is one.
    pub fn next(&mut self) -> Option<&T> {
        if self.current + 1 >= self.items.len() {
            return None;
        }
        self.current += 1;
        self.items.get(self.current)
    }

    /// Drops every item `keep` rejects. The position stays on the current
    /// item, or moves to the closest earlier item when the current one goes.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        let current = self.current;
        let mut index = 0;
        let mut kept_before_current = 0;
        let mut current_kept = false;
        self.items.retain(|item| {
            let kept = keep(item);
            if kept && index < current {
                kept_before_current += 1;
            } else if index == current {
                current_kept = kept;
            }
            index += 1;
            kept
        });
        self.current = if current_kept {
            kept_before_current
        } else {
            kept_before_current.saturating_sub(1)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let mut history = History::<i32>::default();
        assert!(history.is_empty());
        assert_eq!(history.previous(), None);
        assert_eq!(history.next(), None);
    }

    #[test]
    fn undoes_and_redoes_in_order() {
        let mut history = History::default();
        for item in 1..=3 {
            history.add(item);
        }
        assert_eq!(history.previous(), Some(&2));
        assert_eq!(history.previous(), Some(&1));
        assert_eq!(history.previous(), None);
        assert_eq!(history.next(), Some(&2));
        assert_eq!(history.next(), Some(&3));
        assert_eq!(history.next(), None);
    }

    #[test]
    fn adding_after_undo_discards_redo_items() {
        let mut history = History::default();
        for item in 1..=3 {
            history.add(item);
        }
        history.previous();
        history.previous();
        history.add(4);
        assert_eq!(history.next(), None);
        assert_eq!(history.previous(), Some(&1));
        assert_eq!(history.next(), Some(&4));
    }

    #[test]
    fn keeps_only_the_most_recent_items() {
        let mut history = History::default();
        for item in 0..(MAXIMUM_SIZE + 10) {
            history.add(item);
        }
        let mut oldest = None;
        while let Some(item) = history.previous() {
            oldest = Some(*item);
        }
        assert_eq!(oldest, Some(10));
    }

    #[test]
    fn retain_keeps_position_on_current_item() {
        let mut history = History::default();
        for item in 1..=5 {
            history.add(item);
        }
        history.previous();
        history.retain(|item| item % 2 == 1);
        assert_eq!(history.previous(), Some(&1));
        assert_eq!(history.next(), Some(&3));
        assert_eq!(history.next(), Some(&5));
    }
}
