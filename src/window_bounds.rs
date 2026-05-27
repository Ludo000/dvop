//! Window size clamping against the active monitor geometry.
//!
//! Kept separate from `ui` so `settings` can clamp persisted values without a circular dependency.

use gdk4::prelude::*;
use gtk4::prelude::*;

/// Minimum window size — kept in sync with `settings.rs` getters.
pub const MIN_WINDOW_WIDTH: i32 = 400;
pub const MIN_WINDOW_HEIGHT: i32 = 300;

/// Editor area we try to preserve when clamping sidebar width.
pub const MIN_EDITOR_WIDTH: i32 = 200;

/// Minimum width for a Markdown/SVG preview pane inside a tab.
pub const MIN_PREVIEW_PANE_WIDTH: i32 = 120;

/// Minimum height for a stacked Markdown preview pane.
pub const MIN_PREVIEW_PANE_HEIGHT: i32 = 120;

/// Minimum editor height in a stacked Markdown tab.
pub const MIN_MARKDOWN_EDITOR_HEIGHT: i32 = 150;

/// Activity bar + paned handles subtracted from the window width to estimate the notebook width.
const EDITOR_AREA_CHROME_WIDTH: i32 = 64;

/// Header + status bar subtracted from window height for the notebook area.
const EDITOR_AREA_CHROME_HEIGHT: i32 = 120;

/// Side-by-side preview when the clamped window is at least this wide; otherwise stacked.
pub const MARKDOWN_SPLIT_MIN_WINDOW_WIDTH: i32 = 900;

/// How the Markdown preview is arranged inside a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownPreviewLayout {
    /// Editor left, preview right.
    SideBySide,
    /// Editor top, preview bottom (fits narrow screens).
    Stacked,
}

/// Clamps a desired window size to fit within monitor bounds and minimum size.
pub fn clamp_window_dimensions(
    width: i32,
    height: i32,
    max_width: i32,
    max_height: i32,
) -> (i32, i32) {
    let max_width = max_width.max(1);
    let max_height = max_height.max(1);
    let min_width = MIN_WINDOW_WIDTH.min(max_width);
    let min_height = MIN_WINDOW_HEIGHT.min(max_height);
    (
        width.max(min_width).min(max_width),
        height.max(min_height).min(max_height),
    )
}

fn monitor_dimensions(monitor: &gdk4::Monitor) -> (i32, i32) {
    let geometry = monitor.geometry();
    (geometry.width(), geometry.height())
}

fn first_monitor_bounds(display: &gdk4::Display) -> Option<(i32, i32)> {
    let monitor = display
        .monitors()
        .item(0)?
        .downcast::<gdk4::Monitor>()
        .ok()?;
    Some(monitor_dimensions(&monitor))
}

/// Monitor geometry for a realized window, or the display's largest monitor before realization.
pub fn screen_bounds_for_window(window: &gtk4::Window) -> Option<(i32, i32)> {
    let display = gtk4::prelude::WidgetExt::display(window);
    if let Some(surface) = window.surface() {
        if let Some(monitor) = display.monitor_at_surface(&surface) {
            return Some(monitor_dimensions(&monitor));
        }
    }
    first_monitor_bounds(&display)
}

/// Returns the first monitor's size in application pixels.
pub fn primary_screen_bounds() -> Option<(i32, i32)> {
    let display = gdk4::Display::default()?;
    first_monitor_bounds(&display)
}

/// Clamps a stored pair to the current screen.
pub fn clamp_size_to_screen(width: i32, height: i32) -> (i32, i32) {
    clamp_size_to_screen_for_window(width, height, None)
}

pub fn clamp_size_to_screen_for_window(
    width: i32,
    height: i32,
    window: Option<&gtk4::Window>,
) -> (i32, i32) {
    if let Some(window) = window {
        if let Some((max_w, max_h)) = screen_bounds_for_window(window) {
            return clamp_window_dimensions(width, height, max_w, max_h);
        }
    }
    if let Some((max_w, max_h)) = primary_screen_bounds() {
        clamp_window_dimensions(width, height, max_w, max_h)
    } else {
        (width.max(MIN_WINDOW_WIDTH), height.max(MIN_WINDOW_HEIGHT))
    }
}

/// GTK4 has no `resize()`; hide + reset default size + show applies a smaller frame.
pub fn force_gtk_window_size(window: &gtk4::Window, width: i32, height: i32) {
    let visible = window.is_visible();
    if visible {
        window.hide();
    }
    window.set_default_size(width, height);
    if visible {
        window.present();
    }
}

/// Keeps the file sidebar from consuming the whole window after a shrink.
pub fn clamp_file_panel_position(paned: &gtk4::Paned, window_width: i32) {
    let max_panel = (window_width - MIN_EDITOR_WIDTH).max(100);
    let position = paned.position().min(max_panel);
    if position != paned.position() {
        paned.set_position(position);
    }
}

/// Notebook width available to editor tabs (after sidebar + activity bar).
pub fn estimate_editor_area_width(window_width: i32, file_panel_width: i32) -> i32 {
    (window_width - file_panel_width - EDITOR_AREA_CHROME_WIDTH)
        .max(MIN_EDITOR_WIDTH + MIN_PREVIEW_PANE_WIDTH)
}

/// Default split position for side-by-side Markdown/SVG preview panes.
pub fn initial_preview_pane_split(editor_area_width: i32) -> i32 {
    let editor_area_width =
        editor_area_width.max(MIN_EDITOR_WIDTH + MIN_PREVIEW_PANE_WIDTH);
    let split = editor_area_width * 2 / 5;
    split
        .max(MIN_EDITOR_WIDTH)
        .min(editor_area_width - MIN_PREVIEW_PANE_WIDTH)
}

/// Default split position for stacked Markdown preview (editor height).
pub fn initial_stacked_preview_split(editor_area_height: i32) -> i32 {
    let editor_area_height = editor_area_height
        .max(MIN_MARKDOWN_EDITOR_HEIGHT + MIN_PREVIEW_PANE_HEIGHT);
    let split = editor_area_height * 2 / 5;
    split
        .max(MIN_MARKDOWN_EDITOR_HEIGHT)
        .min(editor_area_height - MIN_PREVIEW_PANE_HEIGHT)
}

pub fn estimate_editor_area_height(window_height: i32) -> i32 {
    (window_height - EDITOR_AREA_CHROME_HEIGHT)
        .max(MIN_MARKDOWN_EDITOR_HEIGHT + MIN_PREVIEW_PANE_HEIGHT)
}

pub fn clamped_window_size_for(window: &gtk4::ApplicationWindow) -> (i32, i32) {
    let gtk_window = window.upcast_ref::<gtk4::Window>();
    let width = if gtk_window.width() > 0 {
        gtk_window.width()
    } else {
        gtk_window.default_width()
    };
    let height = if gtk_window.height() > 0 {
        gtk_window.height()
    } else {
        gtk_window.default_height()
    };
    clamp_size_to_screen_for_window(width, height, Some(gtk_window))
}

/// Side-by-side preview needs a wide window; narrow screens use a vertical split instead.
pub fn markdown_preview_layout(window: &gtk4::ApplicationWindow) -> MarkdownPreviewLayout {
    let (max_w, _) = clamped_window_size_for(window);
    if max_w >= MARKDOWN_SPLIT_MIN_WINDOW_WIDTH {
        MarkdownPreviewLayout::SideBySide
    } else {
        MarkdownPreviewLayout::Stacked
    }
}

/// Keeps a tab's preview split from forcing the window wider/taller than the editor area.
pub fn clamp_preview_paned_position(
    paned: &gtk4::Paned,
    editor_area_width: i32,
    editor_area_height: i32,
) {
    use gtk4::Orientation;

    let max_start = match paned.orientation() {
        Orientation::Horizontal => {
            (editor_area_width - MIN_PREVIEW_PANE_WIDTH).max(MIN_EDITOR_WIDTH)
        }
        Orientation::Vertical => (editor_area_height - MIN_PREVIEW_PANE_HEIGHT)
            .max(MIN_MARKDOWN_EDITOR_HEIGHT),
        _ => return,
    };
    if paned.position() > max_start {
        paned.set_position(max_start);
    }
}

#[cfg(test)]
#[path = "../tests/unit/ui/window_bounds_tests.rs"]
mod tests;
