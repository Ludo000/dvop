use crate::window_bounds::*;
use serial_test::serial;

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

#[test]
fn estimate_editor_area_width_subtracts_sidebar_and_chrome() {
    assert_eq!(estimate_editor_area_width(1000, 200), 736);
}

#[test]
fn estimate_editor_area_height_respects_minimum_preview_space() {
    let height = estimate_editor_area_height(800);
    assert!(height >= MIN_MARKDOWN_EDITOR_HEIGHT + MIN_PREVIEW_PANE_HEIGHT);
}

#[test]
fn initial_stacked_preview_split_stays_within_editor_bounds() {
    let split = initial_stacked_preview_split(500);
    assert!(split >= MIN_MARKDOWN_EDITOR_HEIGHT);
    assert!(split <= 500 - MIN_PREVIEW_PANE_HEIGHT);
}

#[test]
fn markdown_layout_threshold_chooses_side_by_side_on_wide_windows() {
    let (width, _) = clamp_window_dimensions(1200, 800, 1280, 720);
    let layout = if width >= MARKDOWN_SPLIT_MIN_WINDOW_WIDTH {
        MarkdownPreviewLayout::SideBySide
    } else {
        MarkdownPreviewLayout::Stacked
    };
    assert_eq!(layout, MarkdownPreviewLayout::SideBySide);
}

#[test]
fn markdown_layout_threshold_chooses_stacked_on_narrow_windows() {
    let (width, _) = clamp_window_dimensions(1920, 1080, 720, 720);
    let layout = if width >= MARKDOWN_SPLIT_MIN_WINDOW_WIDTH {
        MarkdownPreviewLayout::SideBySide
    } else {
        MarkdownPreviewLayout::Stacked
    };
    assert_eq!(layout, MarkdownPreviewLayout::Stacked);
}

#[test]
fn initial_preview_pane_split_reserves_minimum_preview_width() {
    let editor_width = estimate_editor_area_width(1200, 250);
    let split = initial_preview_pane_split(editor_width);
    assert!(split >= MIN_EDITOR_WIDTH);
    assert!(split <= editor_width - MIN_PREVIEW_PANE_WIDTH);
}

#[test]
fn clamp_size_to_screen_applies_minimum_without_monitor_data() {
    let (width, height) = clamp_window_dimensions(50, 40, 10_000, 10_000);
    assert_eq!(width, MIN_WINDOW_WIDTH);
    assert_eq!(height, MIN_WINDOW_HEIGHT);
}

#[test]
fn estimate_editor_area_width_never_drops_below_preview_minimum() {
    let width = estimate_editor_area_width(300, 250);
        assert!(width >= MIN_EDITOR_WIDTH + MIN_PREVIEW_PANE_WIDTH);
}

#[test]
fn initial_preview_pane_split_on_narrow_editor_area_stays_within_bounds() {
    let editor_width = MIN_EDITOR_WIDTH + MIN_PREVIEW_PANE_WIDTH + 40;
    let split = initial_preview_pane_split(editor_width);
    assert!(split >= MIN_EDITOR_WIDTH);
    assert!(split <= editor_width - MIN_PREVIEW_PANE_WIDTH);
}

#[test]
fn markdown_split_threshold_constant_matches_side_by_side_layout() {
    assert_eq!(MARKDOWN_SPLIT_MIN_WINDOW_WIDTH, 900);
}

#[test]
#[serial]
fn clamp_file_panel_position_limits_wide_sidebar_panes() {
    gtk4::test_synced(|| {
        let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        paned.set_position(900);
        clamp_file_panel_position(&paned, 800);
        assert!(paned.position() <= 800 - MIN_EDITOR_WIDTH);
    });
}

#[test]
#[serial]
fn clamp_preview_paned_position_limits_horizontal_split() {
    gtk4::test_synced(|| {
        let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        paned.set_position(900);
        clamp_preview_paned_position(&paned, 700, 600);
        assert!(paned.position() <= 700 - MIN_PREVIEW_PANE_WIDTH);
    });
}

#[test]
#[serial]
fn clamp_preview_paned_position_limits_vertical_split() {
    gtk4::test_synced(|| {
        let paned = gtk4::Paned::new(gtk4::Orientation::Vertical);
        paned.set_position(900);
        clamp_preview_paned_position(&paned, 800, 500);
        assert!(paned.position() <= 500 - MIN_PREVIEW_PANE_HEIGHT);
    });
}

#[test]
fn clamp_size_to_screen_applies_minimum_when_no_monitor_data() {
    let (width, height) = clamp_size_to_screen(50, 40);
    assert_eq!(width, MIN_WINDOW_WIDTH);
    assert_eq!(height, MIN_WINDOW_HEIGHT);
}

#[test]
#[serial]
fn primary_screen_bounds_returns_positive_dimensions_when_available() {
    if let Some((width, height)) = primary_screen_bounds() {
        assert!(width > 0);
        assert!(height > 0);
    }
}
