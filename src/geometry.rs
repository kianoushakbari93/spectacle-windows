//! Rectangle math shared by the window position calculations, the screen
//! detector and the window movers.

use windows::Win32::Foundation::RECT;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn min_x(&self) -> f64 {
        self.x
    }

    pub fn min_y(&self) -> f64 {
        self.y
    }

    pub fn mid_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    pub fn mid_y(&self) -> f64 {
        self.y + self.height / 2.0
    }

    pub fn max_x(&self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(&self) -> f64 {
        self.y + self.height
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn is_empty(&self) -> bool {
        !(self.width > 0.0 && self.height > 0.0)
    }

    pub fn is_finite(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
    }

    /// Whether `other` lies entirely within this rectangle, edges included.
    pub fn contains_rect(&self, other: &Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && other.min_x() >= self.min_x()
            && other.max_x() <= self.max_x()
            && other.min_y() >= self.min_y()
            && other.max_y() <= self.max_y()
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.min_x().max(other.min_x());
        let y = self.min_y().max(other.min_y());
        let max_x = self.max_x().min(other.max_x());
        let max_y = self.max_y().min(other.max_y());
        (max_x > x && max_y > y).then(|| Rect::new(x, y, max_x - x, max_y - y))
    }

    /// Converts between the screen coordinate space Windows uses, where y
    /// grows downwards, and the y-up space the position calculations were
    /// written for. The conversion is its own inverse.
    pub fn flipped(&self) -> Rect {
        Rect::new(self.x, -(self.y + self.height), self.width, self.height)
    }

    pub fn from_win32(rect: &RECT) -> Rect {
        Rect::new(
            f64::from(rect.left),
            f64::from(rect.top),
            f64::from(rect.right) - f64::from(rect.left),
            f64::from(rect.bottom) - f64::from(rect.top),
        )
    }

    /// Rounds to the whole pixels Windows positions windows with.
    pub fn to_win32(self) -> RECT {
        let left = round_to_pixel(self.x);
        let top = round_to_pixel(self.y);
        RECT {
            left,
            top,
            right: left + round_to_pixel(self.width),
            bottom: top + round_to_pixel(self.height),
        }
    }
}

fn round_to_pixel(value: f64) -> i32 {
    let rounded = value.round();
    if rounded >= f64::from(i32::MAX) {
        i32::MAX
    } else if rounded <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        rounded as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flipping_twice_is_identity() {
        let rect = Rect::new(10.0, -20.0, 300.0, 400.0);
        assert_eq!(rect.flipped().flipped(), rect);
    }

    #[test]
    fn flipping_preserves_relative_vertical_order() {
        let upper = Rect::new(0.0, 0.0, 10.0, 10.0);
        let lower = Rect::new(0.0, 100.0, 10.0, 10.0);
        assert!(upper.flipped().y > lower.flipped().y);
    }

    #[test]
    fn contains_rect_includes_edges() {
        let outer = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(outer.contains_rect(&outer));
        assert!(outer.contains_rect(&Rect::new(10.0, 10.0, 90.0, 90.0)));
        assert!(!outer.contains_rect(&Rect::new(10.0, 10.0, 91.0, 90.0)));
    }

    #[test]
    fn intersection_of_disjoint_rects_is_none() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 10.0, 10.0);
        assert_eq!(a.intersection(&b), None);
        assert_eq!(
            a.intersection(&Rect::new(5.0, 5.0, 10.0, 10.0)),
            Some(Rect::new(5.0, 5.0, 5.0, 5.0))
        );
    }

    #[test]
    fn win32_round_trip() {
        let rect = RECT {
            left: -1920,
            top: 40,
            right: 0,
            bottom: 1080,
        };
        assert_eq!(Rect::from_win32(&rect).to_win32(), rect);
    }
}
