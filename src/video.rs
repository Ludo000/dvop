//! # Video Playback — GStreamer-Based Video Player
//!
//! Opens video files (MP4, MKV, WebM, AVI, etc.) in an embedded player tab
//! with play/pause/stop, seek, fullscreen toggle, and volume controls.
//!
//! ## Architecture
//!
//! - **GStreamer pipeline** — `playbin` with a `gtk4paintablesink` for
//!   rendering video frames directly into a GTK4 `Picture` widget.
//! - **Global manager** — `GlobalVideoManager` tracks all active video
//!   pipelines. When a new video starts, the others are paused/stopped
//!   to avoid concurrent playback.
//! - **Keyboard controls** — Space (play/pause), Left/Right (seek ±5s),
//!   F (fullscreen toggle), M (mute toggle).
//!
//! See FEATURES.md: Feature #124 — Video Player
//! See FEATURES.md: Feature #125 — Video Controls

// Video playback functionality for Dvop
// This module handles video file playback using GStreamer

use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, EventControllerKey, GestureClick, Image, Label, MenuButton, Orientation,
    Popover, Scale,
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gstreamer::prelude::*;
use gstreamer::{Pipeline, State};

/// Global video playback manager to coordinate multiple video players
#[derive(Clone)]
struct GlobalVideoManager {
    active_players: Arc<Mutex<Vec<(gstreamer::Pipeline, String)>>>, // (pipeline, unique_id)
    stopped_notifications: Arc<Mutex<Vec<String>>>, // List of player IDs that should be notified of stopping
}

impl GlobalVideoManager {
    fn new() -> Self {
        Self {
            active_players: Arc::new(Mutex::new(Vec::new())),
            stopped_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a new video player pipeline with a unique ID
    fn register_player(&self, pipeline: &gstreamer::Pipeline, player_id: String) {
        let mut players = self.active_players.lock().unwrap();

        // Clean up any pipelines that have been set to NULL state (destroyed)
        let original_count = players.len();
        players.retain(|(p, _)| p.current_state() != gstreamer::State::Null);
        let cleaned_count = original_count - players.len();
        if cleaned_count > 0 {
            println!(
                "Video: Cleaned {} dead players during registration",
                cleaned_count
            );
        }

        // Add the new pipeline with its unique ID
        players.push((pipeline.clone(), player_id.clone()));

        println!(
            "Video: Registered new player. Total active players: {}",
            players.len()
        );
        println!(
            "Video: New pipeline name: {} (ID: {})",
            pipeline.upcast_ref::<gstreamer::Object>().name(),
            player_id
        );
    }

    /// Check if this player was stopped by another and should update its UI
    fn check_and_clear_stop_notification(&self, player_id: &str) -> bool {
        let mut notifications = self.stopped_notifications.lock().unwrap();
        if let Some(pos) = notifications.iter().position(|id| id == player_id) {
            notifications.remove(pos);
            true
        } else {
            false
        }
    }

    /// Stop all other video players except the one that's starting to play
    fn stop_other_players(&self, current_pipeline: &gstreamer::Pipeline, current_player_id: &str) {
        let mut players = self.active_players.lock().unwrap();
        let mut notifications = self.stopped_notifications.lock().unwrap();

        let mut stopped_count = 0;
        let current_name = current_pipeline.upcast_ref::<gstreamer::Object>().name();

        println!(
            "Video: Checking {} registered players for stopping",
            players.len()
        );
        println!(
            "Video: Current pipeline name: {} (ID: {})",
            current_name, current_player_id
        );

        // Clean up dead pipelines and stop others
        players.retain(|(pipeline, player_id)| {
            let pipeline_state = pipeline.current_state();
            let pipeline_name = pipeline.upcast_ref::<gstreamer::Object>().name();

            println!(
                "Video: Checking pipeline '{}' (ID: {}) with state {:?}",
                pipeline_name, player_id, pipeline_state
            );

            // Remove if pipeline is NULL (destroyed)
            if pipeline_state == gstreamer::State::Null {
                println!(
                    "Video: Removing NULL pipeline: {} (ID: {})",
                    pipeline_name, player_id
                );
                return false;
            }

            // Check if this is not the current pipeline
            if player_id != current_player_id {
                // Stop this other player if it's playing
                if pipeline_state == gstreamer::State::Playing {
                    println!(
                        "Video: Stopping other playing video player: {} (ID: {})",
                        pipeline_name, player_id
                    );
                    let _ = pipeline.set_state(gstreamer::State::Paused);

                    // Add to notification list so the player can update its UI
                    notifications.push(player_id.clone());

                    stopped_count += 1;
                } else {
                    println!(
                        "Video: Pipeline '{}' (ID: {}) is not playing (state: {:?}), leaving as-is",
                        pipeline_name, player_id, pipeline_state
                    );
                }
            } else {
                println!(
                    "Video: Pipeline '{}' (ID: {}) is the current one, keeping it",
                    pipeline_name, player_id
                );
            }

            true // Keep this pipeline in the list
        });

        if stopped_count > 0 {
            println!("Video: Stopped {} other video player(s)", stopped_count);
        } else {
            println!("Video: No other playing video players found to stop");
        }
    }

    /// Clean up dead pipelines
    fn cleanup_dead_players(&self) {
        let mut players = self.active_players.lock().unwrap();
        let original_count = players.len();
        players.retain(|(p, _)| p.current_state() != gstreamer::State::Null);
        let cleaned_count = original_count - players.len();
        if cleaned_count > 0 {
            println!("Video: Cleaned up {} dead player(s)", cleaned_count);
        }
    }

    /// Stop all video players associated with a specific file path
    fn stop_players_for_file(&self, file_path: &std::path::Path) {
        let mut players = self.active_players.lock().unwrap();
        let mut notifications = self.stopped_notifications.lock().unwrap();

        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let mut stopped_count = 0;

        println!(
            "Video: Stopping all players for file: {}",
            file_path.display()
        );

        // Find and stop all players associated with this file
        players.retain(|(pipeline, player_id)| {
            let pipeline_state = pipeline.current_state();

            // Remove if pipeline is already NULL (destroyed)
            if pipeline_state == gstreamer::State::Null {
                return false;
            }

            // Check if this player is associated with the file being closed
            if player_id.contains(file_name) {
                println!(
                    "Video: Stopping player for closed file: {} (ID: {})",
                    file_name, player_id
                );

                // Stop the pipeline
                let _ = pipeline.set_state(gstreamer::State::Null);

                // Add to notification list for UI updates
                notifications.push(player_id.clone());

                stopped_count += 1;
                return false; // Remove from active players list
            }

            true // Keep this player
        });

        if stopped_count > 0 {
            println!(
                "Video: Stopped {} player(s) for closed file: {}",
                stopped_count, file_name
            );
        }
    }
}

// Global video manager instance
use once_cell::sync::Lazy;
static GLOBAL_VIDEO_MANAGER: Lazy<GlobalVideoManager> = Lazy::new(GlobalVideoManager::new);

/// Public function to check if a file path represents video content
pub fn is_video_file(path: &std::path::Path) -> bool {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    matches!(
        extension.as_str(),
        "mp4"
            | "mkv"
            | "avi"
            | "mov"
            | "wmv"
            | "flv"
            | "webm"
            | "m4v"
            | "mpg"
            | "mpeg"
            | "3gp"
            | "ogv"
    )
}

/// Public function to stop all video players associated with a specific file path
pub fn stop_video_for_file(file_path: &std::path::Path) {
    GLOBAL_VIDEO_MANAGER.stop_players_for_file(file_path);
}

/// Public function to stop all currently playing video players
/// This should be called when an audio player starts playing
pub fn stop_all_video_players() {
    let players = GLOBAL_VIDEO_MANAGER.active_players.lock().unwrap();
    let mut notifications = GLOBAL_VIDEO_MANAGER.stopped_notifications.lock().unwrap();

    println!("Video: Stopping all {} video players", players.len());

    for (pipeline, player_id) in players.iter() {
        if pipeline.current_state() == gstreamer::State::Playing {
            println!("Video: Stopping video player: {}", player_id);
            let _ = pipeline.set_state(gstreamer::State::Paused);
            notifications.push(player_id.clone());
        }
    }
}

/// Helper function to enter fullscreen mode with a video widget
fn enter_fullscreen<W: IsA<gtk4::Window>>(
    video_picture: &gtk4::Picture,
    _parent_window: &W,
    pipeline: &Pipeline,
    is_playing: &Rc<RefCell<bool>>,
    play_button: &Button,
    player_id: &str,
    duration: &Rc<RefCell<Option<u64>>>,
    current_position: &Rc<RefCell<u64>>,
) {
    // Store the original parent so we can restore it later
    let original_parent = video_picture.parent();

    // Create fullscreen window
    let fullscreen_window = gtk4::Window::new();
    fullscreen_window.set_title(Some("Video - Fullscreen"));
    fullscreen_window.set_decorated(false);

    // Optimize for video playback performance
    fullscreen_window.set_resizable(true);
    fullscreen_window.set_modal(false);
    fullscreen_window.set_focus_visible(false);

    // Create a container for the video in fullscreen
    let fullscreen_box = GtkBox::new(Orientation::Vertical, 0);
    fullscreen_box.set_vexpand(true);
    fullscreen_box.set_hexpand(true);
    fullscreen_box.add_css_class("fullscreen-video");
    fullscreen_box.set_can_focus(false);
    fullscreen_box.set_focusable(false);

    // Move the video picture to the fullscreen window
    video_picture.unparent();
    video_picture.set_can_focus(false);
    video_picture.set_focusable(false);
    fullscreen_box.append(video_picture);
    fullscreen_window.set_child(Some(&fullscreen_box));

    // Present the window first, then fullscreen to avoid rendering issues
    fullscreen_window.present();

    // Delay fullscreen to ensure proper initialization
    let fullscreen_window_fs = fullscreen_window.clone();
    glib::timeout_add_local_once(Duration::from_millis(50), move || {
        fullscreen_window_fs.fullscreen();
    });

    // Add Escape key handler to exit fullscreen
    let key_controller = EventControllerKey::new();
    let fullscreen_window_clone = fullscreen_window.clone();
    let video_picture_restore = video_picture.clone();
    let original_parent_clone = original_parent.clone();
    let pipeline_keys = pipeline.clone();
    let is_playing_keys = is_playing.clone();
    let play_button_keys = play_button.clone();
    let player_id_keys = player_id.to_string();
    let duration_keys = duration.clone();
    let current_position_keys = current_position.clone();

    key_controller.connect_key_pressed(move |_controller, key, _code, _modifier| {
        match key {
            // Escape or F: Exit fullscreen
            gtk4::gdk::Key::Escape | gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                println!("Video: Exiting fullscreen");

                // Restore video to original parent
                video_picture_restore.unparent();
                video_picture_restore.set_can_focus(true);
                video_picture_restore.set_focusable(true);
                if let Some(parent) = &original_parent_clone {
                    if let Some(parent_box) = parent.downcast_ref::<GtkBox>() {
                        parent_box.append(&video_picture_restore);
                    }
                }

                fullscreen_window_clone.close();
                return glib::Propagation::Stop;
            }

            // Space or K: Play/Pause toggle
            gtk4::gdk::Key::space | gtk4::gdk::Key::k | gtk4::gdk::Key::K => {
                let mut playing = is_playing_keys.borrow_mut();
                if *playing {
                    // Pause
                    if pipeline_keys.set_state(State::Paused).is_ok() {
                        *playing = false;
                        let pause_icon = Image::from_icon_name("media-playback-start");
                        play_button_keys.set_child(Some(&pause_icon));
                        play_button_keys.set_tooltip_text(Some("Play"));
                        println!("Video: Paused via keyboard in fullscreen");
                    }
                } else {
                    // Play
                    GLOBAL_VIDEO_MANAGER.stop_other_players(&pipeline_keys, &player_id_keys);
                    crate::audio::stop_all_audio_players();
                    if pipeline_keys.set_state(State::Playing).is_ok() {
                        *playing = true;
                        let play_icon = Image::from_icon_name("media-playback-pause");
                        play_button_keys.set_child(Some(&play_icon));
                        play_button_keys.set_tooltip_text(Some("Pause"));
                        println!("Video: Playing via keyboard in fullscreen");
                    }
                }
                return glib::Propagation::Stop;
            }

            // Left arrow: Seek backward 5 seconds
            gtk4::gdk::Key::Left => {
                let current_pos = *current_position_keys.borrow();
                let new_pos = current_pos.saturating_sub(5);
                let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                let _ = pipeline_keys.seek_simple(
                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                    seek_time,
                );
                println!("Video: Seeked backward to {} seconds (fullscreen)", new_pos);
                return glib::Propagation::Stop;
            }

            // Right arrow: Seek forward 5 seconds
            gtk4::gdk::Key::Right => {
                if let Some(dur) = *duration_keys.borrow() {
                    let current_pos = *current_position_keys.borrow();
                    let new_pos = (current_pos + 5).min(dur);
                    let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                    let _ = pipeline_keys.seek_simple(
                        gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                        seek_time,
                    );
                    println!("Video: Seeked forward to {} seconds (fullscreen)", new_pos);
                }
                return glib::Propagation::Stop;
            }

            // J: Seek backward 10 seconds
            gtk4::gdk::Key::j | gtk4::gdk::Key::J => {
                let current_pos = *current_position_keys.borrow();
                let new_pos = current_pos.saturating_sub(10);
                let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                let _ = pipeline_keys.seek_simple(
                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                    seek_time,
                );
                println!(
                    "Video: Seeked backward 10s to {} seconds (fullscreen)",
                    new_pos
                );
                return glib::Propagation::Stop;
            }

            // L: Seek forward 10 seconds
            gtk4::gdk::Key::l | gtk4::gdk::Key::L => {
                if let Some(dur) = *duration_keys.borrow() {
                    let current_pos = *current_position_keys.borrow();
                    let new_pos = (current_pos + 10).min(dur);
                    let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                    let _ = pipeline_keys.seek_simple(
                        gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                        seek_time,
                    );
                    println!(
                        "Video: Seeked forward 10s to {} seconds (fullscreen)",
                        new_pos
                    );
                }
                return glib::Propagation::Stop;
            }

            // Home or 0: Jump to beginning
            gtk4::gdk::Key::Home | gtk4::gdk::Key::_0 => {
                let seek_time = gstreamer::ClockTime::from_seconds(0);
                let _ = pipeline_keys.seek_simple(
                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                    seek_time,
                );
                println!("Video: Jumped to beginning (fullscreen)");
                return glib::Propagation::Stop;
            }

            // End: Jump to end (or near end)
            gtk4::gdk::Key::End => {
                if let Some(dur) = *duration_keys.borrow() {
                    let seek_time = gstreamer::ClockTime::from_seconds(dur.saturating_sub(1));
                    let _ = pipeline_keys.seek_simple(
                        gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                        seek_time,
                    );
                    println!("Video: Jumped to end (fullscreen)");
                }
                return glib::Propagation::Stop;
            }

            _ => {}
        }
        glib::Propagation::Proceed
    });
    fullscreen_window.add_controller(key_controller);

    // Also restore on window close
    let video_picture_close = video_picture.clone();
    let original_parent_close = original_parent.clone();
    fullscreen_window.connect_close_request(move |_| {
        println!("Video: Fullscreen window closing, restoring video");

        // Restore video to original parent
        video_picture_close.unparent();
        video_picture_close.set_can_focus(true);
        video_picture_close.set_focusable(true);
        if let Some(parent) = &original_parent_close {
            if let Some(parent_box) = parent.downcast_ref::<GtkBox>() {
                parent_box.append(&video_picture_close);
            }
        }

        glib::Propagation::Proceed
    });

    println!("Video: Fullscreen window created and shown");
}

/// Video player widget that provides playback controls and visualization
pub struct VideoPlayer {
    pub widget: GtkBox,
    pipeline: Pipeline,
}

impl VideoPlayer {
    /// Creates a new video player widget for the given video file
    pub fn new(video_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Video: Creating video player for: {}", video_path.display());

        // Don't initialize GStreamer or create pipeline during construction
        // This will be done asynchronously after the widget is displayed

        // Create the main container
        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_top(8);
        main_box.set_margin_bottom(8);
        main_box.set_margin_start(8);
        main_box.set_margin_end(8);
        main_box.set_vexpand(true);
        main_box.set_hexpand(true);

        // Header section with hamburger menu
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        header_box.set_hexpand(true);

        // Empty space to push menu to the right
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header_box.append(&spacer);

        // Hamburger menu
        let menu_button = MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some("Video Options"));
        menu_button.add_css_class("flat");

        // Create popover content with options
        let popover_box = GtkBox::new(Orientation::Vertical, 6);
        popover_box.set_margin_top(6);
        popover_box.set_margin_bottom(6);
        popover_box.set_margin_start(6);
        popover_box.set_margin_end(6);

        // Create and set up popover
        let popover = Popover::new();
        popover.set_child(Some(&popover_box));
        menu_button.set_popover(Some(&popover));

        header_box.append(&menu_button);
        main_box.append(&header_box);

        // Video display area
        let video_box = GtkBox::new(Orientation::Vertical, 0);
        video_box.set_vexpand(true);
        video_box.set_hexpand(true);

        // Create a picture widget for video display
        let video_picture = gtk4::Picture::new();
        video_picture.set_hexpand(true);
        video_picture.set_vexpand(true);
        video_picture.set_can_shrink(true);
        video_picture.set_keep_aspect_ratio(true);

        // Add double-click gesture for fullscreen (will be configured later with pipeline data)
        let double_click_gesture = GestureClick::new();
        double_click_gesture.set_button(1); // Left mouse button
        video_picture.add_controller(double_click_gesture.clone());

        video_box.append(&video_picture);

        // Progress section with seekbar
        let progress_box = GtkBox::new(Orientation::Vertical, 8);

        // Create seekbar
        let seekbar = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        seekbar.set_draw_value(false);
        seekbar.set_hexpand(true);

        // Time labels container
        let time_box = GtkBox::new(Orientation::Horizontal, 0);
        time_box.set_halign(gtk4::Align::Fill);

        let current_time_label = Label::new(Some("0:00"));
        current_time_label.add_css_class("caption");
        current_time_label.set_halign(gtk4::Align::Start);
        time_box.append(&current_time_label);

        // Spacer
        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        time_box.append(&spacer);

        let total_time_label = Label::new(Some("0:00"));
        total_time_label.add_css_class("caption");
        total_time_label.set_halign(gtk4::Align::End);
        time_box.append(&total_time_label);

        progress_box.append(&seekbar);
        progress_box.append(&time_box);
        main_box.append(&video_box);
        main_box.append(&progress_box);

        // Controls section
        let controls_box = GtkBox::new(Orientation::Horizontal, 12);
        controls_box.set_halign(gtk4::Align::Center);

        // Play/Pause button
        let play_button = Button::new();
        let play_icon = Image::from_icon_name("media-playback-start");
        play_button.set_child(Some(&play_icon));
        play_button.add_css_class("pill");
        play_button.set_tooltip_text(Some("Play"));
        play_button.set_size_request(48, 48);

        // Stop button
        let stop_button = Button::new();
        let stop_icon = Image::from_icon_name("media-playback-stop");
        stop_button.set_child(Some(&stop_icon));
        stop_button.add_css_class("pill");
        stop_button.set_tooltip_text(Some("Stop"));
        stop_button.set_size_request(48, 48);

        controls_box.append(&play_button);
        controls_box.append(&stop_button);
        main_box.append(&controls_box);

        // Create a placeholder pipeline that will be initialized asynchronously
        // This prevents blocking during app startup
        println!("Video: Creating placeholder pipeline...");

        // Initialize GStreamer in the background
        gstreamer::init().map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        let pipeline = gstreamer::ElementFactory::make("playbin")
            .property("uri", format!("file://{}", video_path.display()))
            .build()
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| -> Box<dyn std::error::Error> { "Failed to cast to pipeline".into() })?;

        println!("Video: Pipeline created, deferring sink setup...");

        println!("Video: Pipeline created, deferring sink setup...");

        // Defer the sink setup to avoid blocking
        let pipeline_sink_setup = pipeline.clone();
        let video_picture_clone = video_picture.clone();
        glib::timeout_add_local_once(Duration::from_millis(50), move || {
            println!("Video: Setting up video sink asynchronously...");

            // Set up GTK sink - use gtksink as fallback if gtk4paintablesink is not available
            let gtksink_result = gstreamer::ElementFactory::make("gtk4paintablesink").build();

            if let Ok(sink) = gtksink_result {
                println!("Video: gtk4paintablesink created successfully");
                pipeline_sink_setup.set_property("video-sink", &sink);

                // Get the paintable from the sink and set it on the picture widget
                let paintable: gtk4::gdk::Paintable = sink.property("paintable");
                video_picture_clone.set_paintable(Some(&paintable));
            } else {
                println!("Video: gtk4paintablesink not available, trying gtksink...");
                // Fallback to gtksink if gtk4paintablesink is not available
                if let Ok(sink) = gstreamer::ElementFactory::make("gtksink").build() {
                    println!("Video: gtksink created successfully");
                    pipeline_sink_setup.set_property("video-sink", &sink);

                    let widget: gtk4::Widget = sink.property("widget");
                    // Replace the picture with the widget
                    if let Some(parent) = video_picture_clone.parent() {
                        parent.downcast_ref::<GtkBox>().map(|b| {
                            b.remove(&video_picture_clone);
                            b.append(&widget);
                        });
                    }
                } else {
                    println!("Video: Warning - No GTK sink available, video will not display");
                }
            }

            println!("Video: Sink setup complete");
        });

        println!("Video: Pipeline created successfully");

        // Don't set pipeline to PAUSED state synchronously to avoid blocking
        // Instead, prepare it asynchronously after the widget is created
        let pipeline_async = pipeline.clone();
        glib::timeout_add_local_once(Duration::from_millis(100), move || {
            match pipeline_async.set_state(State::Paused) {
                Ok(_) => println!("Video: Pipeline set to PAUSED state"),
                Err(e) => println!("Video: Warning - could not set pipeline to PAUSED: {:?}", e),
            }
        });

        // State tracking
        let current_position = Rc::new(RefCell::new(0u64));
        let duration = Rc::new(RefCell::new(None));
        let is_playing = Rc::new(RefCell::new(false));
        let is_seeking = Rc::new(RefCell::new(false));

        // Create a unique player ID
        use std::time::{SystemTime, UNIX_EPOCH};
        let player_id = format!(
            "player_{}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            video_path.file_name().unwrap_or_default().to_string_lossy()
        );

        // Register this pipeline with the global video manager
        GLOBAL_VIDEO_MANAGER.register_player(&pipeline, player_id.clone());

        // Set up message handling
        let bus = pipeline.bus().unwrap();
        let pipeline_debug = pipeline.clone();

        let _bus_watch = bus
            .add_watch(move |_, msg| {
                use gstreamer::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        println!(
                            "Video Error: {} ({})",
                            err.error(),
                            err.debug().unwrap_or_default()
                        );
                    }
                    MessageView::Warning(warn) => {
                        println!(
                            "Video Warning: {} ({})",
                            warn.error(),
                            warn.debug().unwrap_or_default()
                        );
                    }
                    MessageView::StateChanged(state) => {
                        if msg.src() == Some(pipeline_debug.upcast_ref()) {
                            println!(
                                "Video State changed from {:?} to {:?}",
                                state.old(),
                                state.current()
                            );
                        }
                    }
                    _ => {}
                }
                glib::ControlFlow::Continue
            })
            .expect("Failed to add bus watch");

        println!("Video: Message handling set up");

        // Create the VideoPlayer struct
        let player = VideoPlayer {
            widget: main_box,
            pipeline,
        };

        // Set up play/pause button handler
        let pipeline_play = player.pipeline.clone();
        let is_playing_play = is_playing.clone();
        let play_button_clone = play_button.clone();
        let player_id_play = player_id.clone();

        play_button.connect_clicked(move |_| {
            println!("Video: Play button clicked!");
            let mut playing = is_playing_play.borrow_mut();
            if *playing {
                // Pause
                println!("Video: Pausing playback");
                match pipeline_play.set_state(State::Paused) {
                    Ok(_) => {
                        *playing = false;
                        let pause_icon = Image::from_icon_name("media-playback-start");
                        play_button_clone.set_child(Some(&pause_icon));
                        play_button_clone.set_tooltip_text(Some("Play"));
                        println!("Video: Successfully paused");
                    }
                    Err(e) => {
                        println!("Video: Failed to pause: {:?}", e);
                    }
                }
            } else {
                // Stop all other playing videos before starting this one
                println!("Video: About to stop other players before starting playback");
                GLOBAL_VIDEO_MANAGER.stop_other_players(&pipeline_play, &player_id_play);

                // Also stop all audio players
                println!("Video: Stopping all audio players");
                crate::audio::stop_all_audio_players();

                // Play
                println!("Video: Starting playback");
                match pipeline_play.set_state(State::Playing) {
                    Ok(_) => {
                        *playing = true;
                        let play_icon = Image::from_icon_name("media-playback-pause");
                        play_button_clone.set_child(Some(&play_icon));
                        play_button_clone.set_tooltip_text(Some("Pause"));
                        println!("Video: Successfully started playing");
                    }
                    Err(e) => {
                        println!("Video: Failed to start playing: {:?}", e);
                    }
                }
            }
        });

        // Set up stop button handler
        let pipeline_stop = player.pipeline.clone();
        let is_playing_stop = is_playing.clone();
        let play_button_stop = play_button.clone();
        let current_time_label_stop = current_time_label.clone();
        let seekbar_stop = seekbar.clone();

        stop_button.connect_clicked(clone!(
            #[weak]
            pipeline_stop,
            #[weak]
            is_playing_stop,
            #[weak]
            play_button_stop,
            #[weak]
            current_time_label_stop,
            #[weak]
            seekbar_stop,
            move |_| {
                println!("Video: Stop button clicked!");
                let _ = pipeline_stop.set_state(State::Paused);
                *is_playing_stop.borrow_mut() = false;

                // Reset UI
                let stop_icon = Image::from_icon_name("media-playback-start");
                play_button_stop.set_child(Some(&stop_icon));
                play_button_stop.set_tooltip_text(Some("Play"));
                current_time_label_stop.set_text("0:00");
                seekbar_stop.set_value(0.0);

                // Seek to beginning
                let seek_time = gstreamer::ClockTime::from_seconds(0);
                let _ = pipeline_stop.seek_simple(
                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                    seek_time,
                );

                println!("Video: Playback stopped and UI reset");
            }
        ));

        // Configure double-click gesture for fullscreen with pipeline and state
        let video_picture_for_fullscreen = video_picture.clone();
        let pipeline_fullscreen = player.pipeline.clone();
        let is_playing_fullscreen = is_playing.clone();
        let play_button_fullscreen = play_button.clone();
        let player_id_fullscreen = player_id.clone();
        let duration_fullscreen = duration.clone();
        let current_position_fullscreen = current_position.clone();

        double_click_gesture.connect_pressed(move |gesture, n_press, _x, _y| {
            if n_press == 2 {
                // Double-click
                // Get the window we're in
                if let Some(widget) = gesture.widget() {
                    if let Some(root) = widget.root() {
                        // Check if we're in a fullscreen window (undecorated)
                        if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                            if !window.is_decorated() {
                                // We're in fullscreen, close it
                                println!("Video: Double-click in fullscreen, exiting");
                                window.close();
                                return;
                            }
                        }

                        // Not in fullscreen, enter fullscreen
                        println!("Video: Double-click detected, entering fullscreen");
                        if let Some(app_window) = root.downcast_ref::<gtk4::ApplicationWindow>() {
                            enter_fullscreen(
                                &video_picture_for_fullscreen,
                                app_window,
                                &pipeline_fullscreen,
                                &is_playing_fullscreen,
                                &play_button_fullscreen,
                                &player_id_fullscreen,
                                &duration_fullscreen,
                                &current_position_fullscreen,
                            );
                        } else if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                            enter_fullscreen(
                                &video_picture_for_fullscreen,
                                window,
                                &pipeline_fullscreen,
                                &is_playing_fullscreen,
                                &play_button_fullscreen,
                                &player_id_fullscreen,
                                &duration_fullscreen,
                                &current_position_fullscreen,
                            );
                        }
                    }
                }
            }
        });

        // Set up pipeline volume monitoring from global audio volume
        let pipeline_volume_monitor = player.pipeline.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            let global_volume = crate::audio::get_global_volume();
            pipeline_volume_monitor.set_property("volume", global_volume);

            // Periodically cleanup dead player references
            static mut CLEANUP_COUNTER: u32 = 0;
            unsafe {
                CLEANUP_COUNTER += 1;
                if CLEANUP_COUNTER >= 20 {
                    GLOBAL_VIDEO_MANAGER.cleanup_dead_players();
                    CLEANUP_COUNTER = 0;
                }
            }

            glib::ControlFlow::Continue
        });

        // Set initial volume on the pipeline using global audio volume
        let initial_volume = crate::audio::get_global_volume();
        player.pipeline.set_property("volume", initial_volume);

        println!(
            "Video: Using global audio volume: {:.1}%",
            initial_volume * 100.0
        );

        // Set up seekbar change handler
        let pipeline_seek = player.pipeline.clone();
        let is_seeking_seek = is_seeking.clone();
        let duration_seek = duration.clone();
        seekbar.connect_change_value(move |_, _, value| {
            if let Some(dur) = duration_seek.borrow().as_ref() {
                *is_seeking_seek.borrow_mut() = true;
                let seek_pos_secs = (value / 100.0 * (*dur as f64)) as u64;
                let seek_time = gstreamer::ClockTime::from_seconds(seek_pos_secs);

                match pipeline_seek.seek_simple(
                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                    seek_time,
                ) {
                    Ok(_) => println!("Video: Seeked to {} seconds", seek_pos_secs),
                    Err(e) => println!("Video: Seek failed: {:?}", e),
                }

                let is_seeking_reset = is_seeking_seek.clone();
                glib::timeout_add_local_once(Duration::from_millis(200), move || {
                    *is_seeking_reset.borrow_mut() = false;
                });
            }

            glib::Propagation::Proceed
        });

        // Add keyboard controls for video playback
        let key_controller = EventControllerKey::new();
        let video_picture_keys = video_picture.clone();
        let pipeline_keys = player.pipeline.clone();
        let is_playing_keys = is_playing.clone();
        let play_button_keys = play_button.clone();
        let player_id_keys = player_id.clone();
        let duration_keys = duration.clone();
        let current_position_keys = current_position.clone();

        key_controller.connect_key_pressed(move |_controller, key, _code, _modifier| {
            match key {
                // Space or K: Play/Pause toggle
                gtk4::gdk::Key::space | gtk4::gdk::Key::k | gtk4::gdk::Key::K => {
                    let mut playing = is_playing_keys.borrow_mut();
                    if *playing {
                        // Pause
                        if pipeline_keys.set_state(State::Paused).is_ok() {
                            *playing = false;
                            let pause_icon = Image::from_icon_name("media-playback-start");
                            play_button_keys.set_child(Some(&pause_icon));
                            play_button_keys.set_tooltip_text(Some("Play"));
                            println!("Video: Paused via keyboard");
                        }
                    } else {
                        // Play
                        GLOBAL_VIDEO_MANAGER.stop_other_players(&pipeline_keys, &player_id_keys);
                        crate::audio::stop_all_audio_players();
                        if pipeline_keys.set_state(State::Playing).is_ok() {
                            *playing = true;
                            let play_icon = Image::from_icon_name("media-playback-pause");
                            play_button_keys.set_child(Some(&play_icon));
                            play_button_keys.set_tooltip_text(Some("Pause"));
                            println!("Video: Playing via keyboard");
                        }
                    }
                    return glib::Propagation::Stop;
                }

                // Left arrow: Seek backward 5 seconds
                gtk4::gdk::Key::Left => {
                    let current_pos = *current_position_keys.borrow();
                    let new_pos = current_pos.saturating_sub(5);
                    let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                    let _ = pipeline_keys.seek_simple(
                        gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                        seek_time,
                    );
                    println!("Video: Seeked backward to {} seconds", new_pos);
                    return glib::Propagation::Stop;
                }

                // Right arrow: Seek forward 5 seconds
                gtk4::gdk::Key::Right => {
                    if let Some(dur) = *duration_keys.borrow() {
                        let current_pos = *current_position_keys.borrow();
                        let new_pos = (current_pos + 5).min(dur);
                        let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                        let _ = pipeline_keys.seek_simple(
                            gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                            seek_time,
                        );
                        println!("Video: Seeked forward to {} seconds", new_pos);
                    }
                    return glib::Propagation::Stop;
                }

                // J: Seek backward 10 seconds
                gtk4::gdk::Key::j | gtk4::gdk::Key::J => {
                    let current_pos = *current_position_keys.borrow();
                    let new_pos = current_pos.saturating_sub(10);
                    let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                    let _ = pipeline_keys.seek_simple(
                        gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                        seek_time,
                    );
                    println!("Video: Seeked backward 10s to {} seconds", new_pos);
                    return glib::Propagation::Stop;
                }

                // L: Seek forward 10 seconds
                gtk4::gdk::Key::l | gtk4::gdk::Key::L => {
                    if let Some(dur) = *duration_keys.borrow() {
                        let current_pos = *current_position_keys.borrow();
                        let new_pos = (current_pos + 10).min(dur);
                        let seek_time = gstreamer::ClockTime::from_seconds(new_pos);
                        let _ = pipeline_keys.seek_simple(
                            gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                            seek_time,
                        );
                        println!("Video: Seeked forward 10s to {} seconds", new_pos);
                    }
                    return glib::Propagation::Stop;
                }

                // Home or 0: Jump to beginning
                gtk4::gdk::Key::Home | gtk4::gdk::Key::_0 => {
                    let seek_time = gstreamer::ClockTime::from_seconds(0);
                    let _ = pipeline_keys.seek_simple(
                        gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                        seek_time,
                    );
                    println!("Video: Jumped to beginning");
                    return glib::Propagation::Stop;
                }

                // End: Jump to end (or near end)
                gtk4::gdk::Key::End => {
                    if let Some(dur) = *duration_keys.borrow() {
                        let seek_time = gstreamer::ClockTime::from_seconds(dur.saturating_sub(1));
                        let _ = pipeline_keys.seek_simple(
                            gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                            seek_time,
                        );
                        println!("Video: Jumped to end");
                    }
                    return glib::Propagation::Stop;
                }

                // F: Enter fullscreen
                gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                    if let Some(widget) = _controller.widget() {
                        if let Some(root) = widget.root() {
                            println!("Video: F key pressed, entering fullscreen");
                            if let Some(app_window) = root.downcast_ref::<gtk4::ApplicationWindow>()
                            {
                                enter_fullscreen(
                                    &video_picture_keys,
                                    app_window,
                                    &pipeline_keys,
                                    &is_playing_keys,
                                    &play_button_keys,
                                    &player_id_keys,
                                    &duration_keys,
                                    &current_position_keys,
                                );
                            } else if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                                enter_fullscreen(
                                    &video_picture_keys,
                                    window,
                                    &pipeline_keys,
                                    &is_playing_keys,
                                    &play_button_keys,
                                    &player_id_keys,
                                    &duration_keys,
                                    &current_position_keys,
                                );
                            }
                        }
                    }
                    return glib::Propagation::Stop;
                }

                _ => {}
            }
            glib::Propagation::Proceed
        });

        player.widget.add_controller(key_controller);

        // Set up periodic position updates
        let pipeline_query = player.pipeline.clone();
        let current_position_update = current_position.clone();
        let duration_update = duration.clone();
        let is_seeking_update = is_seeking.clone();
        let current_time_label_update = current_time_label.clone();
        let total_time_label_update = total_time_label.clone();
        let seekbar_update = seekbar.clone();
        let is_playing_timer = is_playing.clone();
        let play_button_timer = play_button.clone();
        let player_id_timer = player_id.clone();

        glib::timeout_add_local(Duration::from_millis(1000), move || {
            // Check if this player was stopped by another player
            if GLOBAL_VIDEO_MANAGER.check_and_clear_stop_notification(&player_id_timer) {
                println!("Video: Received stop notification - updating UI to paused state");
                *is_playing_timer.borrow_mut() = false;
                let play_icon = Image::from_icon_name("media-playback-start");
                play_button_timer.set_child(Some(&play_icon));
                play_button_timer.set_tooltip_text(Some("Play"));
            }

            if *is_seeking_update.borrow() {
                return glib::ControlFlow::Continue;
            }

            // Query position and duration
            if let Some(position) = pipeline_query.query_position::<gstreamer::ClockTime>() {
                let pos_secs = position.seconds();
                *current_position_update.borrow_mut() = pos_secs;
                current_time_label_update.set_text(&format_duration(pos_secs));

                // Update seekbar
                if let Some(dur) = *duration_update.borrow() {
                    if dur > 0 {
                        let progress = (pos_secs as f64 / dur as f64) * 100.0;
                        seekbar_update.set_value(progress);
                    }
                }
            }

            // Query duration (only once)
            if duration_update.borrow().is_none() {
                if let Some(dur) = pipeline_query.query_duration::<gstreamer::ClockTime>() {
                    let dur_secs = dur.seconds();
                    *duration_update.borrow_mut() = Some(dur_secs);
                    total_time_label_update.set_text(&format_duration(dur_secs));
                    println!("Video: Duration detected - {} seconds", dur_secs);
                }
            }

            glib::ControlFlow::Continue
        });

        Ok(player)
    }
}

/// Formats duration in seconds to MM:SS or HH:MM:SS format
fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("movie.mp4")));
        assert!(is_video_file(Path::new("video.avi")));
        assert!(is_video_file(Path::new("clip.mkv")));
        assert!(is_video_file(Path::new("film.mov")));
        assert!(is_video_file(Path::new("show.webm")));
        assert!(is_video_file(Path::new("recording.flv")));
        
        assert!(!is_video_file(Path::new("document.txt")));
        assert!(!is_video_file(Path::new("image.png")));
        assert!(!is_video_file(Path::new("audio.mp3")));
    }

    #[test]
    fn test_format_duration_short() {
        assert_eq!(format_duration(0), "0:00");
        assert_eq!(format_duration(30), "0:30");
        assert_eq!(format_duration(60), "1:00");
        assert_eq!(format_duration(90), "1:30");
        assert_eq!(format_duration(125), "2:05");
    }

    #[test]
    fn test_format_duration_long() {
        assert_eq!(format_duration(3600), "1:00:00");
        assert_eq!(format_duration(3661), "1:01:01");
        assert_eq!(format_duration(7325), "2:02:05");
        assert_eq!(format_duration(86400), "24:00:00");
    }

    #[test]
    fn test_global_video_manager_creation() {
        let manager = GlobalVideoManager::new();
        let players = manager.active_players.lock().unwrap();
        assert_eq!(players.len(), 0);
    }

    #[test]
    fn test_video_manager_stop_notifications() {
        let manager = GlobalVideoManager::new();
        
        // Add a notification
        {
            let mut notifications = manager.stopped_notifications.lock().unwrap();
            notifications.push("player_1".to_string());
        }
        
        // Check and clear it
        assert!(manager.check_and_clear_stop_notification("player_1"));
        
        // Should be cleared now
        assert!(!manager.check_and_clear_stop_notification("player_1"));
    }

    #[test]
    fn test_stop_video_for_file() {
        // This function should be callable without crashing
        let path = Path::new("test.mp4");
        stop_video_for_file(path);
        // Should complete without panic
    }

    #[test]
    fn test_stop_all_video_players() {
        // This function should be callable without crashing
        stop_all_video_players();
        // Should complete without panic
    }
}
