//! Window position calculations, ported line for line from the JavaScript
//! calculations of the macOS version.
//!
//! Every rectangle here is in a y-up coordinate space (y grows upwards, as in
//! Cocoa), which is what the rounding rules below were designed for. Callers
//! convert Windows' y-down rectangles with [`Rect::flipped`] before and after.

use crate::action::WindowAction;
use crate::geometry::Rect;

/// Calculates where a window should go for `action`, or `None` when the
/// action has no calculation or the result is not a usable rectangle.
pub fn calculate_window_rect(
    action: WindowAction,
    window_rect: Rect,
    visible_frame_of_source_screen: Rect,
    visible_frame_of_destination_screen: Rect,
) -> Option<Rect> {
    let screen = visible_frame_of_destination_screen;
    let result = match action {
        WindowAction::Center => center_window_rect_in_visible_frame_of_screen(window_rect, screen),
        WindowAction::Fullscreen => screen,
        WindowAction::LeftHalf => left_half(window_rect, screen),
        WindowAction::RightHalf => right_half(window_rect, screen),
        WindowAction::TopHalf => top_half(window_rect, screen),
        WindowAction::BottomHalf => bottom_half(window_rect, screen),
        WindowAction::UpperLeft => upper_left(window_rect, screen),
        WindowAction::LowerLeft => lower_left(window_rect, screen),
        WindowAction::UpperRight => upper_right(window_rect, screen),
        WindowAction::LowerRight => lower_right(window_rect, screen),
        WindowAction::NextDisplay | WindowAction::PreviousDisplay => {
            move_window_rect_to_display(window_rect, visible_frame_of_source_screen, screen)
        }
        WindowAction::NextThird => find_next_third(window_rect, screen),
        WindowAction::PreviousThird => find_previous_third(window_rect, screen),
        WindowAction::Larger => resize_window_rect(window_rect, screen, 30.0),
        WindowAction::Smaller => resize_window_rect(window_rect, screen, -30.0),
        WindowAction::Undo | WindowAction::Redo => return None,
    };
    (result.is_finite() && !result.is_empty()).then_some(result)
}

/// JavaScript's `Math.round`, which rounds halves towards positive infinity,
/// unlike Rust's `f64::round`.
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

fn rect_centered_within_rect(rect1: Rect, rect2: Rect) -> bool {
    let centered_mid_x = (rect2.mid_x() - rect1.mid_x()).abs() <= 1.0;
    let centered_mid_y = (rect2.mid_y() - rect1.mid_y()).abs() <= 1.0;
    rect1.contains_rect(&rect2) && centered_mid_x && centered_mid_y
}

fn rect_fits_within_rect(rect1: Rect, rect2: Rect) -> bool {
    rect1.width <= rect2.width && rect1.height <= rect2.height
}

fn center_window_rect_in_visible_frame_of_screen(
    window_rect: Rect,
    visible_frame_of_screen: Rect,
) -> Rect {
    let mut calculated = window_rect;
    calculated.x = js_round((visible_frame_of_screen.width - window_rect.width) / 2.0)
        + visible_frame_of_screen.x;
    calculated.y = js_round((visible_frame_of_screen.height - window_rect.height) / 2.0)
        + visible_frame_of_screen.y;
    calculated
}

fn move_window_rect_to_display(window_rect: Rect, _source: Rect, destination: Rect) -> Rect {
    if rect_fits_within_rect(window_rect, destination) {
        center_window_rect_in_visible_frame_of_screen(window_rect, destination)
    } else {
        destination
    }
}

fn left_half(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_half = screen;
    one_half.width = (one_half.width / 2.0).floor();
    if (window_rect.mid_y() - one_half.mid_y()).abs() <= 1.0 {
        let mut two_thirds = one_half;
        two_thirds.width = (screen.width * 2.0 / 3.0).floor();
        if rect_centered_within_rect(one_half, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_half;
            one_third.width = (screen.width / 3.0).floor();
            return one_third;
        }
    }
    one_half
}

fn right_half(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_half = screen;
    one_half.width = (one_half.width / 2.0).floor();
    one_half.x += one_half.width;
    if (window_rect.mid_y() - one_half.mid_y()).abs() <= 1.0 {
        let mut two_thirds = one_half;
        two_thirds.width = (screen.width * 2.0 / 3.0).floor();
        two_thirds.x = screen.x + screen.width - two_thirds.width;
        if rect_centered_within_rect(one_half, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_half;
            one_third.width = (screen.width / 3.0).floor();
            one_third.x = screen.x + screen.width - one_third.width;
            return one_third;
        }
    }
    one_half
}

fn top_half(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_half = screen;
    one_half.height = (one_half.height / 2.0).floor();
    one_half.y += one_half.height + (screen.height % 2.0);
    if (window_rect.mid_x() - one_half.mid_x()).abs() <= 1.0 {
        let mut two_thirds = one_half;
        two_thirds.height = (screen.height * 2.0 / 3.0).floor();
        two_thirds.y = screen.y + screen.height - two_thirds.height;
        if rect_centered_within_rect(one_half, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_half;
            one_third.height = (screen.height / 3.0).floor();
            one_third.y = screen.y + screen.height - one_third.height;
            return one_third;
        }
    }
    one_half
}

fn bottom_half(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_half = screen;
    one_half.height = (one_half.height / 2.0).floor();
    if (window_rect.mid_x() - one_half.mid_x()).abs() <= 1.0 {
        let mut two_thirds = one_half;
        two_thirds.height = (screen.height * 2.0 / 3.0).floor();
        if rect_centered_within_rect(one_half, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_half;
            one_third.height = (screen.height / 3.0).floor();
            return one_third;
        }
    }
    one_half
}

fn upper_left(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_quarter = screen;
    one_quarter.width = (screen.width / 2.0).floor();
    one_quarter.height = (screen.height / 2.0).floor();
    one_quarter.y = screen.y + (screen.height / 2.0).floor() + (screen.height % 2.0);
    if (window_rect.mid_y() - one_quarter.mid_y()).abs() <= 1.0 {
        let mut two_thirds = one_quarter;
        two_thirds.width = (screen.width * 2.0 / 3.0).floor();
        if rect_centered_within_rect(one_quarter, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_quarter;
            one_third.width = (screen.width / 3.0).floor();
            return one_third;
        }
    }
    one_quarter
}

fn lower_left(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_quarter = screen;
    one_quarter.width = (screen.width / 2.0).floor();
    one_quarter.height = (screen.height / 2.0).floor();
    if (window_rect.mid_y() - one_quarter.mid_y()).abs() <= 1.0 {
        let mut two_thirds = one_quarter;
        two_thirds.width = (screen.width * 2.0 / 3.0).floor();
        if rect_centered_within_rect(one_quarter, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_quarter;
            one_third.width = (screen.width / 3.0).floor();
            return one_third;
        }
    }
    one_quarter
}

fn upper_right(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_quarter = screen;
    one_quarter.width = (screen.width / 2.0).floor();
    one_quarter.height = (screen.height / 2.0).floor();
    one_quarter.x += one_quarter.width;
    one_quarter.y = screen.y + (screen.height / 2.0).floor() + (screen.height % 2.0);
    if (window_rect.mid_y() - one_quarter.mid_y()).abs() <= 1.0 {
        let mut two_thirds = one_quarter;
        two_thirds.width = (screen.width * 2.0 / 3.0).floor();
        two_thirds.x = screen.x + screen.width - two_thirds.width;
        if rect_centered_within_rect(one_quarter, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_quarter;
            one_third.width = (screen.width / 3.0).floor();
            one_third.x = screen.x + screen.width - one_third.width;
            return one_third;
        }
    }
    one_quarter
}

fn lower_right(window_rect: Rect, screen: Rect) -> Rect {
    let mut one_quarter = screen;
    one_quarter.width = (screen.width / 2.0).floor();
    one_quarter.height = (screen.height / 2.0).floor();
    one_quarter.x += one_quarter.width;
    if (window_rect.mid_y() - one_quarter.mid_y()).abs() <= 1.0 {
        let mut two_thirds = one_quarter;
        two_thirds.width = (screen.width * 2.0 / 3.0).floor();
        two_thirds.x = screen.x + screen.width - two_thirds.width;
        if rect_centered_within_rect(one_quarter, window_rect) {
            return two_thirds;
        }
        if rect_centered_within_rect(two_thirds, window_rect) {
            let mut one_third = one_quarter;
            one_third.width = (screen.width / 3.0).floor();
            one_third.x = screen.x + screen.width - one_third.width;
            return one_third;
        }
    }
    one_quarter
}

fn thirds_from_visible_frame_of_screen(screen: Rect) -> [Rect; 6] {
    let third_width = (screen.width / 3.0).floor();
    let third_height = (screen.height / 3.0).floor();
    let mut thirds = [screen; 6];
    for (i, third) in thirds.iter_mut().take(3).enumerate() {
        third.x = screen.x + third_width * i as f64;
        third.width = third_width;
    }
    for (i, third) in thirds.iter_mut().skip(3).enumerate() {
        third.y = screen.y + screen.height - third_height * (i as f64 + 1.0);
        third.height = third_height;
    }
    thirds
}

fn find_next_third(window_rect: Rect, screen: Rect) -> Rect {
    let thirds = thirds_from_visible_frame_of_screen(screen);
    thirds
        .iter()
        .position(|third| rect_centered_within_rect(*third, window_rect))
        .map_or(thirds[0], |i| thirds[(i + 1) % thirds.len()])
}

fn find_previous_third(window_rect: Rect, screen: Rect) -> Rect {
    let thirds = thirds_from_visible_frame_of_screen(screen);
    thirds
        .iter()
        .position(|third| rect_centered_within_rect(*third, window_rect))
        .map_or(thirds[0], |i| thirds[(i + thirds.len() - 1) % thirds.len()])
}

fn resize_window_rect(window_rect: Rect, screen: Rect, size_offset: f64) -> Rect {
    let mut resized = window_rect;
    resized.width += size_offset;
    resized.x -= (size_offset / 2.0).floor();
    resized =
        adjusted_window_rect_against_left_and_right_edges_of_screen(window_rect, resized, screen);
    if resized.width >= screen.width {
        resized.width = screen.width;
    }
    resized.height += size_offset;
    resized.y -= (size_offset / 2.0).floor();
    resized =
        adjusted_window_rect_against_top_and_bottom_edges_of_screen(window_rect, resized, screen);
    if resized.height >= screen.height {
        resized.height = screen.height;
        resized.y = window_rect.y;
    }
    if against_all_edges_of_screen(window_rect, screen) && size_offset < 0.0 {
        resized.width = window_rect.width + size_offset;
        resized.x = window_rect.x - (size_offset / 2.0).floor();
        resized.height = window_rect.height + size_offset;
        resized.y = window_rect.y - (size_offset / 2.0).floor();
    }
    if resized_window_rect_is_too_small(resized, screen) {
        resized = window_rect;
    }
    resized
}

fn against_edge_of_screen(gap: f64) -> bool {
    gap.abs() <= 5.0
}

fn against_the_left_edge_of_screen(window_rect: Rect, screen: Rect) -> bool {
    against_edge_of_screen(window_rect.x - screen.x)
}

fn against_the_right_edge_of_screen(window_rect: Rect, screen: Rect) -> bool {
    against_edge_of_screen(window_rect.max_x() - screen.max_x())
}

fn against_the_top_edge_of_screen(window_rect: Rect, screen: Rect) -> bool {
    against_edge_of_screen(window_rect.max_y() - screen.max_y())
}

fn against_the_bottom_edge_of_screen(window_rect: Rect, screen: Rect) -> bool {
    against_edge_of_screen(window_rect.y - screen.y)
}

fn against_all_edges_of_screen(window_rect: Rect, screen: Rect) -> bool {
    against_the_left_edge_of_screen(window_rect, screen)
        && against_the_right_edge_of_screen(window_rect, screen)
        && against_the_top_edge_of_screen(window_rect, screen)
        && against_the_bottom_edge_of_screen(window_rect, screen)
}

fn adjusted_window_rect_against_left_and_right_edges_of_screen(
    original: Rect,
    resized: Rect,
    screen: Rect,
) -> Rect {
    let mut adjusted = resized;
    if against_the_right_edge_of_screen(original, screen) {
        adjusted.x = screen.max_x() - adjusted.width;
        if against_the_left_edge_of_screen(original, screen) {
            adjusted.width = screen.width;
        }
    }
    if against_the_left_edge_of_screen(original, screen) {
        adjusted.x = screen.x;
    }
    adjusted
}

fn adjusted_window_rect_against_top_and_bottom_edges_of_screen(
    original: Rect,
    resized: Rect,
    screen: Rect,
) -> Rect {
    let mut adjusted = resized;
    if against_the_top_edge_of_screen(original, screen) {
        adjusted.y = screen.max_y() - adjusted.height;
        if against_the_bottom_edge_of_screen(original, screen) {
            adjusted.height = screen.height;
        }
    }
    if against_the_bottom_edge_of_screen(original, screen) {
        adjusted.y = screen.y;
    }
    adjusted
}

fn resized_window_rect_is_too_small(window_rect: Rect, screen: Rect) -> bool {
    let minimum_width = (screen.width / 4.0).floor();
    let minimum_height = (screen.height / 4.0).floor();
    window_rect.width <= minimum_width || window_rect.height <= minimum_height
}

#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
