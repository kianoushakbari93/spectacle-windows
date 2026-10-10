//! Generated from the macOS SpectacleSpecs window calculation specs; every
//! expectation of the original Specta suite is preserved here.

use super::*;

fn calculate(action: WindowAction, window: Rect, source: Rect, destination: Rect) -> Rect {
    calculate_window_rect(action, window, source, destination)
        .unwrap_or_else(|| panic!("{action:?} produced no rectangle for {window:?}"))
}

#[test]
/// SpectacleBottomHalfWindowCalculation: should calculate a window's CGRect in the bottom half of the screen
fn a_window_in_the_bottom_half_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::BottomHalf,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 1440.0, 436.0)
    );
}

#[test]
/// SpectacleBottomHalfWindowCalculation: should calculate a window's CGRect in the bottom 2/3 of the screen
fn a_window_in_the_bottom_2_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::BottomHalf,
            Rect::new(0.0, 4.0, 1440.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 1440.0, 582.0)
    );
}

#[test]
/// SpectacleBottomHalfWindowCalculation: should calculate a window's CGRect in the bottom 1/3 of the screen
fn a_window_in_the_bottom_1_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::BottomHalf,
            Rect::new(0.0, 4.0, 1440.0, 582.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectacleCenterWindowCalculation: should calculate a centered window's CGRect
fn a_centered_window() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Center,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(438.0, 249.0, 564.0, 384.0)
    );
}

#[test]
/// SpectacleFullscreenWindowCalculation: should calculate a fullscreen window's CGRect
fn a_fullscreen_window() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Fullscreen,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 1440.0, 873.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when centered in the screen
fn a_window_s_larger_rect_when_centered_in_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(360.0, 222.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(345.0, 207.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(345.0, 207.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(330.0, 192.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(330.0, 192.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(315.0, 177.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the top edge of the screen
fn a_window_s_larger_rect_when_against_the_top_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(402.0, 441.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(387.0, 411.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(387.0, 411.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(372.0, 381.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(372.0, 381.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(357.0, 351.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the bottom edge of the screen
fn a_window_s_larger_rect_when_against_the_bottom_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(238.0, 4.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(223.0, 4.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(223.0, 4.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(208.0, 4.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(208.0, 4.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(193.0, 4.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the left edge of the screen
fn a_window_s_larger_rect_when_against_the_left_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 250.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 235.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 235.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(0.0, 220.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 220.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(0.0, 205.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the right edge of the screen
fn a_window_s_larger_rect_when_against_the_right_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(720.0, 303.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(690.0, 288.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(690.0, 288.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(660.0, 273.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(660.0, 273.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(630.0, 258.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the top and left edges of the screen
fn a_window_s_larger_rect_when_against_the_top_and_left_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 441.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 411.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 411.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(0.0, 381.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 381.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(0.0, 351.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the top and right edges of the screen
fn a_window_s_larger_rect_when_against_the_top_and_right_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(720.0, 441.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(690.0, 411.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(690.0, 411.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(660.0, 381.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(660.0, 381.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(630.0, 351.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the bottom and left edges of the screen
fn a_window_s_larger_rect_when_against_the_bottom_and_left_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 3.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 4.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 4.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the bottom and right edges of the screen
fn a_window_s_larger_rect_when_against_the_bottom_and_right_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(720.0, 4.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(690.0, 4.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(690.0, 4.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(660.0, 4.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(660.0, 4.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(630.0, 4.0, 810.0, 526.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the top and bottom edges of the screen
fn a_window_s_larger_rect_when_against_the_top_and_bottom_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(344.0, 4.0, 720.0, 873.0),
            source,
            destination
        ),
        Rect::new(329.0, 4.0, 750.0, 873.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(329.0, 4.0, 750.0, 873.0),
            source,
            destination
        ),
        Rect::new(314.0, 4.0, 780.0, 873.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(314.0, 4.0, 780.0, 873.0),
            source,
            destination
        ),
        Rect::new(299.0, 4.0, 810.0, 873.0)
    );
}

#[test]
/// SpectacleLargerWindowCalculation: should calculate a window's larger CGRect when against the left and right edges of the screen
fn a_window_s_larger_rect_when_against_the_left_and_right_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 285.0, 1436.0, 446.0),
            source,
            destination
        ),
        Rect::new(0.0, 270.0, 1440.0, 476.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 270.0, 1440.0, 476.0),
            source,
            destination
        ),
        Rect::new(0.0, 255.0, 1440.0, 506.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Larger,
            Rect::new(0.0, 255.0, 1440.0, 506.0),
            source,
            destination
        ),
        Rect::new(0.0, 240.0, 1440.0, 536.0)
    );
}

#[test]
/// SpectacleLeftHalfWindowCalculation: should calculate a window's CGRect in the left half of the screen
fn a_window_in_the_left_half_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::LeftHalf,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 720.0, 873.0)
    );
}

#[test]
/// SpectacleLeftHalfWindowCalculation: should calculate a window's CGRect in the left 2/3 of the screen
fn a_window_in_the_left_2_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::LeftHalf,
            Rect::new(0.0, 4.0, 720.0, 873.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 960.0, 873.0)
    );
}

#[test]
/// SpectacleLeftHalfWindowCalculation: should calculate a window's CGRect in the left 1/3 of the screen
fn a_window_in_the_left_1_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::LeftHalf,
            Rect::new(0.0, 4.0, 960.0, 873.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectacleLowerLeftWindowCalculation: should calculate a window's CGRect in the lower left corner of the screen
fn a_window_in_the_lower_left_corner_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::LowerLeft,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleLowerRightWindowCalculation: should calculate a window's CGRect in the lower right corner of the screen
fn a_window_in_the_lower_right_corner_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::LowerRight,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(720.0, 4.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleNextDisplayWindowCalculation: should calculate a centered window's CGRect in the next display
fn a_centered_window_in_the_next_display() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(1440.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextDisplay,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(1878.0, 249.0, 564.0, 384.0)
    );
}

#[test]
/// SpectacleNextThirdWindowCalculation: should calculate a window's CGRect in the first horizontal third of the screen
fn a_window_in_the_first_horizontal_third_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextThird,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectacleNextThirdWindowCalculation: should calculate a window's CGRect in the second horizontal third of the screen
fn a_window_in_the_second_horizontal_third_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextThird,
            Rect::new(0.0, 4.0, 480.0, 873.0),
            source,
            destination
        ),
        Rect::new(480.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectacleNextThirdWindowCalculation: should calculate a window's CGRect in the last horizontal third of the screen
fn a_window_in_the_last_horizontal_third_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextThird,
            Rect::new(480.0, 4.0, 480.0, 873.0),
            source,
            destination
        ),
        Rect::new(960.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectacleNextThirdWindowCalculation: should calculate a window's CGRect in the first vertical third of the screen
fn a_window_in_the_first_vertical_third_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextThird,
            Rect::new(960.0, 4.0, 480.0, 873.0),
            source,
            destination
        ),
        Rect::new(0.0, 586.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectacleNextThirdWindowCalculation: should calculate a window's CGRect in the second vertical third of the screen
fn a_window_in_the_second_vertical_third_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextThird,
            Rect::new(0.0, 586.0, 1440.0, 291.0),
            source,
            destination
        ),
        Rect::new(0.0, 295.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectacleNextThirdWindowCalculation: should calculate a window's CGRect in the last vertical third of the screen
fn a_window_in_the_last_vertical_third_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::NextThird,
            Rect::new(0.0, 295.0, 1440.0, 291.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectaclePreviousDisplayWindowCalculation: should calculate a centered window's CGRect in the previous display
fn a_centered_window_in_the_previous_display() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(1440.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousDisplay,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(1878.0, 249.0, 564.0, 384.0)
    );
}

#[test]
/// SpectaclePreviousThirdWindowCalculation: should calculate a window's CGRect in the first horizontal third of the screen
fn a_window_in_the_first_horizontal_third_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousThird,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectaclePreviousThirdWindowCalculation: should calculate a window's CGRect in the last vertical third of the screen
fn a_window_in_the_last_vertical_third_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousThird,
            Rect::new(0.0, 4.0, 480.0, 873.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectaclePreviousThirdWindowCalculation: should calculate a window's CGRect in the second vertical third of the screen
fn a_window_in_the_second_vertical_third_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousThird,
            Rect::new(0.0, 4.0, 1440.0, 291.0),
            source,
            destination
        ),
        Rect::new(0.0, 295.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectaclePreviousThirdWindowCalculation: should calculate a window's CGRect in the first vertical third of the screen
fn a_window_in_the_first_vertical_third_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousThird,
            Rect::new(0.0, 295.0, 1440.0, 291.0),
            source,
            destination
        ),
        Rect::new(0.0, 586.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectaclePreviousThirdWindowCalculation: should calculate a window's CGRect in the last horizontal third of the screen
fn a_window_in_the_last_horizontal_third_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousThird,
            Rect::new(0.0, 586.0, 1440.0, 291.0),
            source,
            destination
        ),
        Rect::new(960.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectaclePreviousThirdWindowCalculation: should calculate a window's CGRect in the second horizontal third of the screen
fn a_window_in_the_second_horizontal_third_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::PreviousThird,
            Rect::new(960.0, 4.0, 480.0, 873.0),
            source,
            destination
        ),
        Rect::new(480.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectacleRightHalfWindowCalculation: should calculate a window's CGRect in the right half of the screen
fn a_window_in_the_right_half_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::RightHalf,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(720.0, 4.0, 720.0, 873.0)
    );
}

#[test]
/// SpectacleRightHalfWindowCalculation: should calculate a window's CGRect in the right 2/3 of the screen
fn a_window_in_the_right_2_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::RightHalf,
            Rect::new(720.0, 4.0, 720.0, 873.0),
            source,
            destination
        ),
        Rect::new(480.0, 4.0, 960.0, 873.0)
    );
}

#[test]
/// SpectacleRightHalfWindowCalculation: should calculate a window's CGRect in the right 1/3 of the screen
fn a_window_in_the_right_1_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::RightHalf,
            Rect::new(480.0, 4.0, 960.0, 873.0),
            source,
            destination
        ),
        Rect::new(960.0, 4.0, 480.0, 873.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when centered in the screen
fn a_window_s_smaller_rect_when_centered_in_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(315.0, 177.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(330.0, 192.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(330.0, 192.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(345.0, 207.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(345.0, 207.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(360.0, 222.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the top edge of the screen
fn a_window_s_smaller_rect_when_against_the_top_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(357.0, 351.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(372.0, 381.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(372.0, 381.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(387.0, 411.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(387.0, 411.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(402.0, 441.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the bottom edge of the screen
fn a_window_s_smaller_rect_when_against_the_bottom_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(193.0, 4.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(208.0, 4.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(208.0, 4.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(223.0, 4.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(223.0, 4.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(238.0, 4.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the left edge of the screen
fn a_window_s_smaller_rect_when_against_the_left_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 205.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(0.0, 220.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 220.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(0.0, 235.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 235.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(0.0, 250.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the right edge of the screen
fn a_window_s_smaller_rect_when_against_the_right_edge_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(630.0, 258.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(660.0, 273.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(660.0, 273.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(690.0, 288.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(690.0, 288.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(720.0, 303.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the top and left edges of the screen
fn a_window_s_smaller_rect_when_against_the_top_and_left_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 351.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(0.0, 381.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 381.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(0.0, 411.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 411.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(0.0, 441.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the top and right edges of the screen
fn a_window_s_smaller_rect_when_against_the_top_and_right_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(630.0, 351.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(660.0, 381.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(660.0, 381.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(690.0, 411.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(690.0, 411.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(720.0, 441.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the bottom and left edges of the screen
fn a_window_s_smaller_rect_when_against_the_bottom_and_left_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 4.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 4.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 4.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(0.0, 4.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the bottom and right edges of the screen
fn a_window_s_smaller_rect_when_against_the_bottom_and_right_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(630.0, 4.0, 810.0, 526.0),
            source,
            destination
        ),
        Rect::new(660.0, 4.0, 780.0, 496.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(660.0, 4.0, 780.0, 496.0),
            source,
            destination
        ),
        Rect::new(690.0, 4.0, 750.0, 466.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(690.0, 4.0, 750.0, 466.0),
            source,
            destination
        ),
        Rect::new(720.0, 4.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the top and bottom edges of the screen
fn a_window_s_smaller_rect_when_against_the_top_and_bottom_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(299.0, 4.0, 810.0, 873.0),
            source,
            destination
        ),
        Rect::new(314.0, 4.0, 780.0, 873.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(314.0, 4.0, 780.0, 873.0),
            source,
            destination
        ),
        Rect::new(329.0, 4.0, 750.0, 873.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(329.0, 4.0, 750.0, 873.0),
            source,
            destination
        ),
        Rect::new(344.0, 4.0, 720.0, 873.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against the left and right edges of the screen
fn a_window_s_smaller_rect_when_against_the_left_and_right_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 240.0, 1440.0, 536.0),
            source,
            destination
        ),
        Rect::new(0.0, 255.0, 1440.0, 506.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 255.0, 1440.0, 506.0),
            source,
            destination
        ),
        Rect::new(0.0, 270.0, 1440.0, 476.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 270.0, 1440.0, 476.0),
            source,
            destination
        ),
        Rect::new(0.0, 285.0, 1440.0, 446.0)
    );
}

#[test]
/// SpectacleSmallerWindowCalculation: should calculate a window's smaller CGRect when against all edges of the screen
fn a_window_s_smaller_rect_when_against_all_edges_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(0.0, 4.0, 1440.0, 873.0),
            source,
            destination
        ),
        Rect::new(15.0, 19.0, 1410.0, 843.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(15.0, 19.0, 1410.0, 843.0),
            source,
            destination
        ),
        Rect::new(30.0, 34.0, 1380.0, 813.0)
    );
    assert_eq!(
        calculate(
            WindowAction::Smaller,
            Rect::new(30.0, 34.0, 1380.0, 813.0),
            source,
            destination
        ),
        Rect::new(45.0, 49.0, 1350.0, 783.0)
    );
}

#[test]
/// SpectacleTopHalfWindowCalculation: should calculate a window's CGRect in the top half of the screen
fn a_window_in_the_top_half_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::TopHalf,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 441.0, 1440.0, 436.0)
    );
}

#[test]
/// SpectacleTopHalfWindowCalculation: should calculate a window's CGRect in the top 2/3 of the screen
fn a_window_in_the_top_2_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::TopHalf,
            Rect::new(0.0, 441.0, 1440.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 295.0, 1440.0, 582.0)
    );
}

#[test]
/// SpectacleTopHalfWindowCalculation: should calculate a window's CGRect in the top 1/3 of the screen
fn a_window_in_the_top_1_3_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::TopHalf,
            Rect::new(0.0, 295.0, 1440.0, 582.0),
            source,
            destination
        ),
        Rect::new(0.0, 586.0, 1440.0, 291.0)
    );
}

#[test]
/// SpectacleUpperLeftWindowCalculation: should calculate a window's CGRect in the upper left corner of the screen
fn a_window_in_the_upper_left_corner_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::UpperLeft,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(0.0, 441.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleUpperLeftWindowCalculation: should calculate a window's CGRect in the left 2/3 of the screen
fn a_window_in_the_left_2_3_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::UpperLeft,
            Rect::new(0.0, 441.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 441.0, 960.0, 436.0)
    );
}

#[test]
/// SpectacleUpperLeftWindowCalculation: should calculate a window's CGRect in the left 1/3 of the screen
fn a_window_in_the_left_1_3_of_the_screen_2() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::UpperLeft,
            Rect::new(0.0, 441.0, 960.0, 436.0),
            source,
            destination
        ),
        Rect::new(0.0, 441.0, 480.0, 436.0)
    );
}

#[test]
/// SpectacleUpperRightWindowCalculation: should calculate a window's CGRect in the upper right corner of the screen
fn a_window_in_the_upper_right_corner_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::UpperRight,
            Rect::new(165.0, 245.0, 564.0, 384.0),
            source,
            destination
        ),
        Rect::new(720.0, 441.0, 720.0, 436.0)
    );
}

#[test]
/// SpectacleUpperRightWindowCalculation: should calculate a window's CGRect in the upper right 2/3 corner of the screen
fn a_window_in_the_upper_right_2_3_corner_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::UpperRight,
            Rect::new(720.0, 441.0, 720.0, 436.0),
            source,
            destination
        ),
        Rect::new(480.0, 441.0, 960.0, 436.0)
    );
}

#[test]
/// SpectacleUpperRightWindowCalculation: should calculate a window's CGRect in the upper right 1/3 corner of the screen
fn a_window_in_the_upper_right_1_3_corner_of_the_screen() {
    let source = Rect::new(0.0, 4.0, 1440.0, 873.0);
    let destination = Rect::new(0.0, 4.0, 1440.0, 873.0);
    assert_eq!(
        calculate(
            WindowAction::UpperRight,
            Rect::new(480.0, 441.0, 960.0, 436.0),
            source,
            destination
        ),
        Rect::new(960.0, 441.0, 480.0, 436.0)
    );
}
