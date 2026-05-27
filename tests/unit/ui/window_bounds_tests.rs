use crate::window_bounds::*;

#[test]
fn clamp_window_dimensions_fits_within_screen() {
    let (w, h) = clamp_window_dimensions(1920, 1080, 1280, 720);
    assert_eq!(w, 1280);
    assert_eq!(h, 720);
}

#[test]
fn clamp_window_dimensions_enforces_minimum() {
    let (w, h) = clamp_window_dimensions(100, 50, 1280, 720);
    assert_eq!(w, MIN_WINDOW_WIDTH);
    assert_eq!(h, MIN_WINDOW_HEIGHT);
}

#[test]
fn clamp_window_dimensions_on_tiny_screen_uses_screen_as_cap() {
    let (w, h) = clamp_window_dimensions(2000, 1500, 320, 240);
    assert_eq!(w, 320);
    assert_eq!(h, 240);
}

#[test]
fn clamp_window_dimensions_unchanged_when_already_fits() {
    let (w, h) = clamp_window_dimensions(800, 600, 1280, 720);
    assert_eq!(w, 800);
    assert_eq!(h, 600);
}

#[test]
fn clamp_window_dimensions_on_720_square_screen() {
    let (w, h) = clamp_window_dimensions(1920, 1080, 720, 720);
    assert_eq!(w, 720);
    assert_eq!(h, 720);
}

#[test]
fn clamp_window_dimensions_720_screen_keeps_smaller_saved_size() {
    let (w, h) = clamp_window_dimensions(640, 480, 720, 720);
    assert_eq!(w, 640);
    assert_eq!(h, 480);
}

#[test]
fn initial_preview_pane_split_fits_720_editor_area() {
    let split = initial_preview_pane_split(estimate_editor_area_width(720, 100));
    assert!(split <= 720 - MIN_PREVIEW_PANE_WIDTH);
    assert!(split >= MIN_EDITOR_WIDTH);
}

#[test]
fn markdown_uses_stacked_layout_on_720_screen() {
    let (w, h) = clamp_window_dimensions(1920, 1080, 720, 720);
    assert!(w < MARKDOWN_SPLIT_MIN_WINDOW_WIDTH);
    let split = initial_stacked_preview_split(estimate_editor_area_height(h));
    assert!(split >= MIN_MARKDOWN_EDITOR_HEIGHT);
    assert!(split <= h - MIN_PREVIEW_PANE_HEIGHT);
}
