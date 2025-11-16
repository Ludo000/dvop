// Audio playback functionality for Dvop
// This module handles audio file playback using GStreamer

use glib::clone;
use gtk4::cairo::{Context, Format, ImageSurface};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DrawingArea, GestureClick, Image, Label, MenuButton, Orientation,
    Popover,
};
use rustfft::{num_complex::Complex, FftPlanner};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::settings;
use gstreamer::prelude::*;
use gstreamer::{Pipeline, State};

/// Progress tracking for spectrogram generation
#[derive(Debug, Clone)]
enum SpectrogramProgress {
    NotStarted,
    InProgress(u8),            // Progress percentage
    Complete(SpectrogramData), // Completed spectrogram data
    Error(()),                 // Error occurred
}

/// Thread-safe waveform data for volume visualization
#[derive(Debug, Clone)]
struct WaveformData {
    samples: Vec<f32>, // Peak values for each time segment
    #[allow(dead_code)]
    sample_rate: u32,
    #[allow(dead_code)]
    duration_secs: f64,
}

/// Thread-safe spectrogram data that can be sent between threads
#[derive(Debug, Clone)]
struct SpectrogramData {
    width: usize,
    height: usize,
    pixel_data: Vec<u8>, // RGB pixel data
}

/// Global volume manager to sync volume across all audio players
#[derive(Clone)]
struct GlobalVolumeManager {
    current_volume: Arc<Mutex<f64>>,
}

impl GlobalVolumeManager {
    fn new() -> Self {
        let initial_volume = settings::get_settings().get_audio_volume();
        Self {
            current_volume: Arc::new(Mutex::new(initial_volume)),
        }
    }

    fn get_volume(&self) -> f64 {
        *self.current_volume.lock().unwrap()
    }

    fn set_volume(&self, volume: f64) {
        let clamped_volume = volume.max(0.0).min(1.0);
        *self.current_volume.lock().unwrap() = clamped_volume;

        // Save to settings
        {
            let mut settings = settings::get_settings_mut();
            settings.set_audio_volume(clamped_volume);
            if let Err(e) = settings.save() {
                println!("Audio: Warning - could not save volume setting: {}", e);
            }
        }

        // Refresh settings to trigger any global updates
        settings::refresh_settings();
    }
}

// Global volume manager instance
use once_cell::sync::Lazy;
static GLOBAL_VOLUME_MANAGER: Lazy<GlobalVolumeManager> = Lazy::new(GlobalVolumeManager::new);

/// Global audio playback manager to coordinate multiple audio players
#[derive(Clone)]
struct GlobalAudioManager {
    active_players: Arc<Mutex<Vec<(gstreamer::Pipeline, String, bool)>>>, // (pipeline, unique_id, is_music)
    stopped_notifications: Arc<Mutex<Vec<String>>>, // List of player IDs that should be notified of stopping
}

impl GlobalAudioManager {
    fn new() -> Self {
        Self {
            active_players: Arc::new(Mutex::new(Vec::new())),
            stopped_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a new audio player pipeline with a unique ID
    fn register_player(&self, pipeline: &gstreamer::Pipeline, player_id: String, is_music: bool) {
        let mut players = self.active_players.lock().unwrap();

        // Clean up any pipelines that have been set to NULL state (destroyed) BEFORE adding new one
        let original_count = players.len();
        players.retain(|(p, _, _)| p.current_state() != gstreamer::State::Null);
        let cleaned_count = original_count - players.len();
        if cleaned_count > 0 {
            println!(
                "Audio: Cleaned {} dead players during registration",
                cleaned_count
            );
        }

        // Add the new pipeline with its unique ID and music flag
        players.push((pipeline.clone(), player_id.clone(), is_music));

        println!(
            "Audio: Registered new player. Total active players: {}",
            players.len()
        );
        println!(
            "Audio: New pipeline name: {} (ID: {}, music: {})",
            pipeline.upcast_ref::<gstreamer::Object>().name(),
            player_id,
            is_music
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

    /// Stop all other audio players except the one that's starting to play
    fn stop_other_players(&self, current_pipeline: &gstreamer::Pipeline, current_player_id: &str) {
        let mut players = self.active_players.lock().unwrap();
        let mut notifications = self.stopped_notifications.lock().unwrap();

        let mut stopped_count = 0;
        let current_name = current_pipeline.upcast_ref::<gstreamer::Object>().name();

        println!(
            "Audio: Checking {} registered players for stopping",
            players.len()
        );
        println!(
            "Audio: Current pipeline name: {} (ID: {})",
            current_name, current_player_id
        );

        // Clean up dead pipelines and stop others
        players.retain(|(pipeline, player_id, _is_music)| {
            let pipeline_state = pipeline.current_state();
            let pipeline_name = pipeline.upcast_ref::<gstreamer::Object>().name();

            println!(
                "Audio: Checking pipeline '{}' (ID: {}) with state {:?}",
                pipeline_name, player_id, pipeline_state
            );

            // Remove if pipeline is NULL (destroyed)
            if pipeline_state == gstreamer::State::Null {
                println!(
                    "Audio: Removing NULL pipeline: {} (ID: {})",
                    pipeline_name, player_id
                );
                return false;
            }

            // Check if this is not the current pipeline
            if player_id != current_player_id {
                // Stop this other player if it's playing
                if pipeline_state == gstreamer::State::Playing {
                    println!(
                        "Audio: Stopping other playing audio player: {} (ID: {})",
                        pipeline_name, player_id
                    );
                    let _ = pipeline.set_state(gstreamer::State::Paused);

                    // Add to notification list so the player can update its UI
                    notifications.push(player_id.clone());

                    stopped_count += 1;
                } else {
                    println!(
                        "Audio: Pipeline '{}' (ID: {}) is not playing (state: {:?}), leaving as-is",
                        pipeline_name, player_id, pipeline_state
                    );
                }
            } else {
                println!(
                    "Audio: Pipeline '{}' (ID: {}) is the current one, keeping it",
                    pipeline_name, player_id
                );
            }

            true // Keep this pipeline in the list
        });

        if stopped_count > 0 {
            println!("Audio: Stopped {} other audio player(s)", stopped_count);
        } else {
            println!("Audio: No other playing audio players found to stop");
        }
    }

    /// Clean up dead pipelines
    fn cleanup_dead_players(&self) {
        let mut players = self.active_players.lock().unwrap();
        let original_count = players.len();
        players.retain(|(p, _, _)| p.current_state() != gstreamer::State::Null);
        let cleaned_count = original_count - players.len();
        if cleaned_count > 0 {
            println!("Audio: Cleaned up {} dead player(s)", cleaned_count);
        }
    }

    /// Stop all audio players associated with a specific file path
    /// This is used when a music file tab is closed
    fn stop_players_for_file(&self, file_path: &std::path::Path) {
        let mut players = self.active_players.lock().unwrap();
        let mut notifications = self.stopped_notifications.lock().unwrap();

        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let mut stopped_count = 0;

        println!(
            "Audio: Stopping all players for file: {}",
            file_path.display()
        );

        // Find and stop all players associated with this file
        players.retain(|(pipeline, player_id, _is_music)| {
            let pipeline_state = pipeline.current_state();

            // Remove if pipeline is already NULL (destroyed)
            if pipeline_state == gstreamer::State::Null {
                return false;
            }

            // Check if this player is associated with the file being closed
            if player_id.contains(file_name) {
                println!(
                    "Audio: Stopping player for closed file: {} (ID: {})",
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
                "Audio: Stopped {} player(s) for closed file: {}",
                stopped_count, file_name
            );
        }
    }
}

// Global audio manager instance
static GLOBAL_AUDIO_MANAGER: Lazy<GlobalAudioManager> = Lazy::new(GlobalAudioManager::new);

/// Public function to update global volume from UI components
pub fn set_global_volume(volume: f64) {
    GLOBAL_VOLUME_MANAGER.set_volume(volume);
}

/// Public function to get current global volume
#[allow(dead_code)]
pub fn get_global_volume() -> f64 {
    GLOBAL_VOLUME_MANAGER.get_volume()
}

/// Public function to check if a file path represents music content
pub fn is_music_file(path: &std::path::Path) -> bool {
    is_music_content(path)
}

/// Public function to stop all audio players associated with a specific file path
/// This should be called when a music file tab is closed
pub fn stop_audio_for_file(file_path: &std::path::Path) {
    GLOBAL_AUDIO_MANAGER.stop_players_for_file(file_path);
}

/// Public function to stop all currently playing audio players
/// This should be called when a video starts playing
pub fn stop_all_audio_players() {
    let players = GLOBAL_AUDIO_MANAGER.active_players.lock().unwrap();
    let mut notifications = GLOBAL_AUDIO_MANAGER.stopped_notifications.lock().unwrap();

    println!("Audio: Stopping all {} audio players", players.len());

    for (pipeline, player_id, _is_music) in players.iter() {
        if pipeline.current_state() == gstreamer::State::Playing {
            println!("Audio: Stopping audio player: {}", player_id);
            let _ = pipeline.set_state(gstreamer::State::Paused);
            notifications.push(player_id.clone());
        }
    }
}

/// Determines if an audio file is likely to be music content
/// This is a heuristic-based approach using file extension and path analysis
fn is_music_content(audio_path: &Path) -> bool {
    let path_str = audio_path.to_string_lossy().to_lowercase();
    let file_name = audio_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Check for music-specific file extensions
    if let Some(extension) = audio_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        match ext.as_str() {
            // Typically music formats
            "mp3" | "flac" | "m4a" | "aac" | "opus" | "wav" | "ogg" | "wma" => return true,
            _ => return false,
        }
    }

    // Check for non-music indicators first (higher priority)
    let non_music_indicators = [
        "speech",
        "voice",
        "talk",
        "podcast",
        "interview",
        "notification",
        "alert",
        "system",
        "effect",
        "sfx",
        "recording",
        "memo",
        "note",
        "call",
        "voicemail",
        "announcement",
        "dialog",
        "dialogue",
    ];

    for indicator in &non_music_indicators {
        if path_str.contains(indicator) || file_name.contains(indicator) {
            return false;
        }
    }

    // Check for music-related keywords in path or filename
    let music_indicators = [
        "music",
        "song",
        "track",
        "album",
        "artist",
        "band",
        "mp3",
        "audio",
        "sound",
        "playlist",
        "library",
        "tune",
        "melody",
        "beat",
        "rhythm",
        "genre",
        "acoustic",
        "instrumental",
    ];

    // Check for music indicators
    for indicator in &music_indicators {
        if path_str.contains(indicator) || file_name.contains(indicator) {
            return true;
        }
    }

    // Check for common music file naming patterns (Artist - Title, etc.)
    if file_name.contains(" - ") || file_name.contains("_-_") {
        return true;
    }

    // Check for track numbers at the beginning (e.g., "01. Song Name" or "1 - Song")
    if file_name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
        && (file_name.contains(". ") || file_name.contains(" - ")) {
            return true;
        }

    // Default assumption: if it's a common audio format in a typical location,
    // it's likely music
    if let Some(extension) = audio_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "wav" | "ogg" | "wma")
    } else {
        false
    }
}

/// Audio player widget that provides playback controls and visualization
pub struct AudioPlayer {
    pub widget: GtkBox,
    pipeline: Pipeline,
    #[allow(dead_code)]
    waveform_area: DrawingArea,
    #[allow(dead_code)]
    play_button: Button,
    #[allow(dead_code)]
    current_position: Rc<RefCell<u64>>,
    #[allow(dead_code)]
    duration: Rc<RefCell<Option<u64>>>,
    #[allow(dead_code)]
    is_playing: Rc<RefCell<bool>>,
    #[allow(dead_code)]
    spectrogram_data: Rc<RefCell<Option<ImageSurface>>>,
    #[allow(dead_code)]
    spectrum_area: DrawingArea,
    #[allow(dead_code)]
    waveform_data: Rc<RefCell<Option<WaveformData>>>,
}

impl AudioPlayer {
    /// Creates a new audio player widget for the given audio file
    pub fn new(audio_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Audio: Initializing GStreamer...");
        // Initialize GStreamer
        gstreamer::init().map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        println!("Audio: Creating audio player for: {}", audio_path.display());

        // Determine if this is music content
        let is_music = is_music_content(audio_path);
        println!(
            "Audio: Content type detected: {} (is_music: {})",
            if is_music { "Music" } else { "Non-music audio" },
            is_music
        );

        // Create the main container
        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_top(20);
        main_box.set_margin_bottom(20);
        main_box.set_margin_start(20);
        main_box.set_margin_end(20);
        main_box.set_valign(gtk4::Align::Center);
        main_box.set_halign(gtk4::Align::Fill);

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
        menu_button.set_tooltip_text(Some("Audio Options"));
        menu_button.add_css_class("flat");

        // Create popover content with spectrum button
        let popover_box = GtkBox::new(Orientation::Vertical, 6);
        popover_box.set_margin_top(6);
        popover_box.set_margin_bottom(6);
        popover_box.set_margin_start(6);
        popover_box.set_margin_end(6);

        // Add spectrum/spectrogram button to popover
        let spectrum_button = Button::with_label("Generate Spectrogram");
        spectrum_button.set_halign(gtk4::Align::Fill);
        popover_box.append(&spectrum_button);

        // Create and set up popover
        let popover = Popover::new();
        popover.set_child(Some(&popover_box));
        menu_button.set_popover(Some(&popover));

        header_box.append(&menu_button);
        main_box.append(&header_box);

        // File info section
        let info_box = GtkBox::new(Orientation::Vertical, 8);

        // Audio file icon
        let audio_icon = Image::from_icon_name("audio-x-generic");
        audio_icon.set_pixel_size(64);
        audio_icon.set_margin_bottom(12);
        info_box.append(&audio_icon);

        // File name label
        let filename = audio_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown Audio File");
        let filename_label = Label::new(Some(filename));
        filename_label.add_css_class("title-2");
        filename_label.set_margin_bottom(4);
        info_box.append(&filename_label);

        // File path label
        let path_label = Label::new(Some(&format!("Path: {}", audio_path.display())));
        path_label.add_css_class("caption");
        path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        info_box.append(&path_label);

        main_box.append(&info_box);

        // Progress section - replace slider with waveform visualization
        let progress_box = GtkBox::new(Orientation::Vertical, 8);

        // Waveform area (replaces position scale)
        let waveform_area = DrawingArea::new();
        waveform_area.set_size_request(400, 60);
        waveform_area.set_hexpand(true);
        waveform_area.add_css_class("waveform-timeline");

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

        progress_box.append(&waveform_area);
        progress_box.append(&time_box);
        main_box.append(&progress_box);

        // Spectrum visualization section
        let spectrum_box = GtkBox::new(Orientation::Vertical, 8);
        spectrum_box.set_visible(false); // Initially hidden until spectrogram is generated

        let spectrum_label = Label::new(Some("Frequency Spectrum"));
        spectrum_label.add_css_class("caption");
        spectrum_label.set_halign(gtk4::Align::Center);
        spectrum_box.append(&spectrum_label);

        // Create spectrum drawing area
        let spectrum_area = DrawingArea::new();
        spectrum_area.set_size_request(400, 100);
        spectrum_area.set_hexpand(true);
        spectrum_area.add_css_class("spectrum-visualizer");
        spectrum_box.append(&spectrum_area);

        // Controls section
        let controls_box = GtkBox::new(Orientation::Horizontal, 6);
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

        main_box.append(&spectrum_box);

        // Create simple GStreamer pipeline for audio playback
        let uri = format!("file://{}", audio_path.display());
        println!("Audio: Creating simple playback pipeline for URI: {}", uri);

        let pipeline = gstreamer::ElementFactory::make("playbin")
            .property("uri", &uri)
            .build()
            .unwrap()
            .downcast::<gstreamer::Pipeline>()
            .unwrap();

        println!("Audio: Pipeline created successfully");

        // Try to set pipeline to PAUSED state to prepare it and get duration
        match pipeline.set_state(State::Paused) {
            Ok(_) => println!("Audio: Pipeline set to PAUSED state"),
            Err(e) => println!("Audio: Warning - could not set pipeline to PAUSED: {:?}", e),
        }

        // State tracking
        let current_position = Rc::new(RefCell::new(0u64));
        let duration = Rc::new(RefCell::new(None));
        let is_playing = Rc::new(RefCell::new(false));
        let pending_seek_position = Rc::new(RefCell::new(None::<u64>));
        let is_seeking = Rc::new(RefCell::new(false));

        // Create a unique player ID for this audio player
        use std::time::{SystemTime, UNIX_EPOCH};
        let player_id = format!(
            "player_{}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            audio_path.file_name().unwrap_or_default().to_string_lossy()
        );

        // Register this pipeline with the global audio manager
        GLOBAL_AUDIO_MANAGER.register_player(&pipeline, player_id.clone(), is_music);

        // Waveform and spectrogram data
        let waveform_data = Rc::new(RefCell::new(None));
        let spectrogram_data = Rc::new(RefCell::new(None));
        let audio_path_for_spectrogram = audio_path.to_path_buf();

        // Set up simple message handling
        let bus = pipeline.bus().unwrap();
        let pipeline_debug = pipeline.clone();

        let _bus_watch = bus
            .add_watch(move |_, msg| {
                use gstreamer::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        println!(
                            "Audio Error: {} ({})",
                            err.error(),
                            err.debug().unwrap_or_default()
                        );
                    }
                    MessageView::Warning(warn) => {
                        println!(
                            "Audio Warning: {} ({})",
                            warn.error(),
                            warn.debug().unwrap_or_default()
                        );
                    }
                    MessageView::StateChanged(state) => {
                        if msg.src() == Some(pipeline_debug.upcast_ref()) {
                            println!(
                                "Audio State changed from {:?} to {:?}",
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

        println!("Audio: Message handling set up");

        // Create the AudioPlayer struct
        let player = AudioPlayer {
            widget: main_box,
            pipeline,
            waveform_area: waveform_area.clone(),
            play_button: play_button.clone(),
            current_position: current_position.clone(),
            duration: duration.clone(),
            is_playing: is_playing.clone(),
            spectrogram_data: spectrogram_data.clone(),
            spectrum_area: spectrum_area.clone(),
            waveform_data: waveform_data.clone(),
        };

        // Set up play/pause button handler
        let pipeline_play = player.pipeline.clone();
        let is_playing_play = is_playing.clone();
        let play_button_clone = play_button.clone();
        let pending_seek_play = pending_seek_position.clone();
        let waveform_area_play = waveform_area.clone();
        let current_position_play = current_position.clone();
        let duration_play = duration.clone();
        let is_seeking_play = is_seeking.clone();
        let player_id_play = player_id.clone();

        play_button.connect_clicked(move |_| {
            println!("Audio: Play button clicked!");
            let mut playing = is_playing_play.borrow_mut();
            if *playing {
                // Pause
                println!("Audio: Pausing playback");
                match pipeline_play.set_state(State::Paused) {
                    Ok(_) => {
                        *playing = false;
                        let pause_icon = Image::from_icon_name("media-playback-start");
                        play_button_clone.set_child(Some(&pause_icon));
                        play_button_clone.set_tooltip_text(Some("Play"));
                        println!("Audio: Successfully paused");
                    }
                    Err(e) => {
                        println!("Audio: Failed to pause: {:?}", e);
                    }
                }
            } else {
                // Stop all other playing audio before starting this one
                println!("Audio: About to stop other players before starting playback");
                GLOBAL_AUDIO_MANAGER.stop_other_players(&pipeline_play, &player_id_play);

                // Stop all video players as well
                println!("Audio: Stopping all video players");
                crate::video::stop_all_video_players();

                // Play
                println!("Audio: Starting playback");
                match pipeline_play.set_state(State::Playing) {
                    Ok(_) => {
                        *playing = true;
                        let play_icon = Image::from_icon_name("media-playback-pause");
                        play_button_clone.set_child(Some(&play_icon));
                        play_button_clone.set_tooltip_text(Some("Pause"));
                        println!("Audio: Successfully started playing");

                        // Apply pending seek position if there is one
                        if let Some(seek_pos) = *pending_seek_play.borrow() {
                            println!("Audio: Found pending seek position: {} seconds", seek_pos);
                            let pipeline_seek_delayed = pipeline_play.clone();
                            let pending_seek_delayed = pending_seek_play.clone();
                            let waveform_area_delayed = waveform_area_play.clone();
                            let current_position_delayed = current_position_play.clone();
                            let duration_delayed = duration_play.clone();
                            let is_seeking_delayed = is_seeking_play.clone();

                            // Set seeking flag to prevent position updates from interfering
                            *is_seeking_delayed.borrow_mut() = true;

                            // Use a short delay to ensure the pipeline is fully playing
                            glib::timeout_add_local_once(Duration::from_millis(100), move || {
                                println!("Audio: Applying delayed seek to {} seconds", seek_pos);
                                let seek_time = gstreamer::ClockTime::from_seconds(seek_pos);
                                match pipeline_seek_delayed.seek_simple(
                                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                                    seek_time,
                                ) {
                                    Ok(_) => {
                                        println!(
                                            "Audio: Delayed seek successful to {} seconds",
                                            seek_pos
                                        );
                                        *pending_seek_delayed.borrow_mut() = None; // Clear pending seek
                                        *current_position_delayed.borrow_mut() = seek_pos; // Update current position

                                        // Update waveform area to trigger redraw at correct position
                                        if let Some(_dur) = *duration_delayed.borrow() {
                                            waveform_area_delayed.queue_draw();
                                        }

                                        // Clear seeking flag after a short delay
                                        let is_seeking_reset = is_seeking_delayed.clone();
                                        glib::timeout_add_local_once(
                                            Duration::from_millis(200),
                                            move || {
                                                *is_seeking_reset.borrow_mut() = false;
                                            },
                                        );
                                    }
                                    Err(e) => {
                                        println!("Audio: Delayed seek failed: {:?}", e);
                                        *is_seeking_delayed.borrow_mut() = false;
                                    }
                                }
                            });
                        }
                    }
                    Err(e) => {
                        println!("Audio: Failed to start playing: {:?}", e);
                    }
                }
            }
        });

        // Set up stop button handler
        let pipeline_stop = player.pipeline.clone();
        let is_playing_stop = is_playing.clone();
        let play_button_stop = play_button.clone();
        let waveform_area_stop = waveform_area.clone();
        let current_time_label_stop = current_time_label.clone();
        let pending_seek_stop = pending_seek_position.clone();

        stop_button.connect_clicked(clone!(
            #[weak]
            pipeline_stop,
            #[weak]
            is_playing_stop,
            #[weak]
            play_button_stop,
            #[weak]
            waveform_area_stop,
            #[weak]
            current_time_label_stop,
            #[weak]
            pending_seek_stop,
            move |_| {
                println!("Audio: Stop button clicked!");
                let _ = pipeline_stop.set_state(State::Paused); // Use PAUSED instead of READY to maintain duration info
                *is_playing_stop.borrow_mut() = false;
                *pending_seek_stop.borrow_mut() = None; // Clear any pending seek

                // Reset UI
                let stop_icon = Image::from_icon_name("media-playback-start");
                play_button_stop.set_child(Some(&stop_icon));
                play_button_stop.set_tooltip_text(Some("Play"));
                waveform_area_stop.queue_draw(); // Redraw waveform at position 0
                current_time_label_stop.set_text("0:00");

                // Seek to beginning
                let seek_time = gstreamer::ClockTime::from_seconds(0);
                let _ = pipeline_stop.seek_simple(
                    gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                    seek_time,
                );

                println!("Audio: Playback stopped and UI reset");
            }
        ));

        // Set up pipeline volume monitoring from global volume and cleanup dead players
        let pipeline_volume = player.pipeline.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            let global_volume = GLOBAL_VOLUME_MANAGER.get_volume();

            // Update pipeline volume to match global volume
            pipeline_volume.set_property("volume", global_volume);

            // Periodically cleanup dead player references (every ~10 seconds)
            static mut CLEANUP_COUNTER: u32 = 0;
            unsafe {
                CLEANUP_COUNTER += 1;
                if CLEANUP_COUNTER >= 20 {
                    // Every 20 * 500ms = 10 seconds
                    GLOBAL_AUDIO_MANAGER.cleanup_dead_players();
                    CLEANUP_COUNTER = 0;
                }
            }

            glib::ControlFlow::Continue
        });

        // Set initial volume on the pipeline using global volume
        let initial_volume = GLOBAL_VOLUME_MANAGER.get_volume();
        player.pipeline.set_property("volume", initial_volume);

        println!(
            "Audio: Volume control initialized with global volume: {:.1}%",
            initial_volume * 100.0
        );

        // Set up spectrum button handler
        let spectrogram_data_button = spectrogram_data.clone();
        let spectrum_area_button = spectrum_area.clone();
        let spectrum_box_button = spectrum_box.clone();
        let audio_path_button = audio_path_for_spectrogram.clone();

        spectrum_button.connect_clicked(move |button| {
            println!("Audio: Generate Spectrogram button clicked!");

            // Disable button and change text to show it's processing
            button.set_sensitive(false);
            button.set_label("Generating...");

            // Clone data for the background thread
            let spectrogram_data_gen = spectrogram_data_button.clone();
            let spectrum_area_gen = spectrum_area_button.clone();
            let spectrum_box_gen = spectrum_box_button.clone();
            let audio_path_clone = audio_path_button.clone();
            let button_clone = button.clone();

            // Start spectrogram generation in background thread
            let progress_data = Arc::new(Mutex::new(SpectrogramProgress::NotStarted));
            let progress_data_thread = progress_data.clone();

            std::thread::spawn(move || {
                println!("Audio: Starting background spectrogram generation...");
                *progress_data_thread.lock().unwrap() = SpectrogramProgress::InProgress(0);

                match generate_spectrogram_simple(&audio_path_clone, progress_data_thread.clone()) {
                    Ok(spectrogram) => {
                        *progress_data_thread.lock().unwrap() =
                            SpectrogramProgress::Complete(spectrogram);
                    }
                    Err(e) => {
                        println!("Audio: Spectrogram generation failed: {}", e);
                        *progress_data_thread.lock().unwrap() = SpectrogramProgress::Error(());
                    }
                }
            });

            // Check progress periodically and update UI
            let progress_check = progress_data.clone();
            glib::timeout_add_local(Duration::from_millis(200), move || {
                let progress = progress_check.lock().unwrap();
                match &*progress {
                    SpectrogramProgress::Complete(data) => {
                        println!("Audio: Spectrogram generation completed");
                        // Create Cairo surface from RGB data in main thread
                        if let Ok(surface) = create_surface_from_data(data) {
                            *spectrogram_data_gen.borrow_mut() = Some(surface);
                            spectrum_area_gen.queue_draw();
                        }
                        // Show the spectrum visualization section
                        spectrum_box_gen.set_visible(true);
                        // Re-enable button
                        button_clone.set_sensitive(true);
                        button_clone.set_label("Regenerate Spectrogram");
                        glib::ControlFlow::Break // Stop the timer
                    }
                    SpectrogramProgress::Error(_) => {
                        println!("Audio: Spectrogram generation failed, showing placeholder");
                        // Generate placeholder in main thread
                        if let Ok(placeholder) = generate_placeholder_spectrogram() {
                            *spectrogram_data_gen.borrow_mut() = Some(placeholder);
                            spectrum_area_gen.queue_draw();
                        }
                        // Show the spectrum visualization section even with placeholder
                        spectrum_box_gen.set_visible(true);
                        // Re-enable button
                        button_clone.set_sensitive(true);
                        button_clone.set_label("Generate Spectrogram");
                        glib::ControlFlow::Break
                    }
                    SpectrogramProgress::InProgress(percent) => {
                        if *percent % 20 == 0 && *percent > 0 {
                            button_clone.set_label(&format!("Generating... {}%", percent));
                        }
                        glib::ControlFlow::Continue
                    }
                    SpectrogramProgress::NotStarted => glib::ControlFlow::Continue,
                }
            });
        });

        // Generate waveform data automatically when player is created
        // Use a simple placeholder first, then try to generate real waveform asynchronously
        let waveform_data_gen = waveform_data.clone();
        let waveform_area_gen = waveform_area.clone();
        let audio_path_waveform = audio_path_for_spectrogram.clone();

        // Set a simple placeholder waveform immediately
        *waveform_data_gen.borrow_mut() = Some(WaveformData {
            samples: generate_placeholder_waveform_pattern(),
            sample_rate: 44100,
            duration_secs: 180.0, // Default duration
        });
        waveform_area_gen.queue_draw();

        // Try to generate a better waveform without threading issues
        glib::timeout_add_local_once(Duration::from_millis(1000), move || {
            println!("Audio: Generating better waveform...");
            match generate_waveform_simple_safe(&audio_path_waveform) {
                Ok(waveform) => {
                    println!(
                        "Audio: Generated better waveform with {} samples",
                        waveform.samples.len()
                    );
                    *waveform_data_gen.borrow_mut() = Some(waveform);
                    waveform_area_gen.queue_draw();
                }
                Err(e) => {
                    println!(
                        "Audio: Could not generate better waveform: {}, keeping placeholder",
                        e
                    );
                }
            }
        });

        // Set up waveform area mouse click handler for seeking
        let click_gesture = GestureClick::new();
        let pipeline_seek = player.pipeline.clone();
        let duration_seek = duration.clone();
        let is_seeking_clone = is_seeking.clone();
        let current_time_label_seek = current_time_label.clone();
        let is_playing_seek = is_playing.clone();
        let pending_seek_seek = pending_seek_position.clone();
        let waveform_area_seek = waveform_area.clone();
        let current_position_seek = current_position.clone();

        click_gesture.connect_pressed(clone!(
            #[weak] pipeline_seek,
            #[weak] duration_seek,
            #[weak] is_seeking_clone,
            #[weak] current_time_label_seek,
            #[weak] is_playing_seek,
            #[weak] pending_seek_seek,
            #[weak] waveform_area_seek,
            #[weak] current_position_seek,
            move |_, _, x, _| {
                println!("Audio: Waveform clicked at x: {}", x);
                let duration_val = *duration_seek.borrow();
                
                // Get waveform width and calculate time position
                let widget_width = waveform_area_seek.width() as f64;
                if widget_width > 0.0 {
                    let time_progress = x / widget_width;
                    
                    if let Some(dur) = duration_val {
                        *is_seeking_clone.borrow_mut() = true;
                        let seek_pos_secs = (time_progress * dur as f64) as u64;
                        println!("Audio: Seeking to position: {} seconds", seek_pos_secs);
                        
                        // Update time label immediately
                        current_time_label_seek.set_text(&format_duration(seek_pos_secs));
                        
                        // Update the current position immediately for visual feedback
                        *current_position_seek.borrow_mut() = seek_pos_secs;
                        
                        if *is_playing_seek.borrow() {
                            // Pipeline is playing/paused, seek immediately
                            let seek_time = gstreamer::ClockTime::from_seconds(seek_pos_secs);
                            match pipeline_seek.seek_simple(
                                gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                                seek_time,
                            ) {
                                Ok(_) => {
                                    println!("Audio: Immediate seek successful");
                                    // Clear pending seek since we seeked successfully
                                    *pending_seek_seek.borrow_mut() = None;
                                }
                                Err(e) => {
                                    println!("Audio: Immediate seek failed: {:?}", e);
                                    // Store for later if immediate seek failed
                                    *pending_seek_seek.borrow_mut() = Some(seek_pos_secs);
                                }
                            }
                        } else {
                            // Pipeline is stopped, store position for later
                            println!("Audio: Pipeline stopped, storing seek position {} seconds for later", seek_pos_secs);
                            *pending_seek_seek.borrow_mut() = Some(seek_pos_secs);
                        }
                        
                        // Trigger waveform redraw to show new position immediately
                        waveform_area_seek.queue_draw();
                        
                        // Reset seeking flag after a delay
                        let is_seeking_reset = is_seeking_clone.clone();
                        glib::timeout_add_local_once(Duration::from_millis(200), move || {
                            *is_seeking_reset.borrow_mut() = false;
                        });
                    } else if let Some(dur) = pipeline_seek.query_duration::<gstreamer::ClockTime>() {
                        let dur_secs = dur.seconds();
                        *duration_seek.borrow_mut() = Some(dur_secs);
                        println!("Audio: Directly queried duration: {} seconds", dur_secs);
                        
                        let seek_pos_secs = (time_progress * dur_secs as f64) as u64;
                        current_time_label_seek.set_text(&format_duration(seek_pos_secs));
                        
                        // Update current position for immediate visual feedback
                        *current_position_seek.borrow_mut() = seek_pos_secs;
                        
                        if !*is_playing_seek.borrow() {
                            *pending_seek_seek.borrow_mut() = Some(seek_pos_secs);
                        }
                        waveform_area_seek.queue_draw();
                    }
                }
            }
        ));

        waveform_area.add_controller(click_gesture);

        // Set up periodic position updates with a simpler approach
        println!("Audio: Setting up position update timer...");

        let waveform_area_update = waveform_area.clone();
        let pipeline_query = player.pipeline.clone();
        let current_position_update = current_position.clone();
        let duration_update = duration.clone();
        let is_seeking_update = is_seeking.clone();
        let current_time_label_update = current_time_label.clone();
        let total_time_label_update = total_time_label.clone();
        let pending_seek_timer = pending_seek_position.clone();
        let is_playing_timer = is_playing.clone();
        let play_button_timer = play_button.clone();
        let player_id_timer = player_id.clone();

        // Use a much simpler timeout approach
        let _timeout = glib::timeout_add_local(Duration::from_millis(1000), move || {
            // Check if this player was stopped by another player
            if GLOBAL_AUDIO_MANAGER.check_and_clear_stop_notification(&player_id_timer) {
                println!("Audio: Received stop notification - updating UI to paused state");
                *is_playing_timer.borrow_mut() = false;
                let play_icon = Image::from_icon_name("media-playback-start");
                play_button_timer.set_child(Some(&play_icon));
                play_button_timer.set_tooltip_text(Some("Play"));
            }

            println!("Audio: Timer callback executing...");

            if *is_seeking_update.borrow() {
                println!("Audio: Skipping update - currently seeking");
                return glib::ControlFlow::Continue;
            }

            // Don't update waveform display if there's a pending seek position (user has clicked waveform manually)
            if pending_seek_timer.borrow().is_some() {
                println!("Audio: Skipping waveform update - pending seek exists");
                // Still update duration and time labels, just not the waveform position
                if duration_update.borrow().is_none() {
                    if let Some(dur) = pipeline_query.query_duration::<gstreamer::ClockTime>() {
                        let dur_secs = dur.seconds();
                        *duration_update.borrow_mut() = Some(dur_secs);
                        total_time_label_update.set_text(&format_duration(dur_secs));
                        println!("Audio: Duration detected - {} seconds", dur_secs);
                    }
                }
                return glib::ControlFlow::Continue;
            }

            // Try to query position and duration directly
            if let Some(position) = pipeline_query.query_position::<gstreamer::ClockTime>() {
                let pos_secs = position.seconds();
                *current_position_update.borrow_mut() = pos_secs;
                current_time_label_update.set_text(&format_duration(pos_secs));
                println!("Audio: Position update - {} seconds", pos_secs);
            } else {
                println!("Audio: Could not query position");
            }

            // Query duration (only once)
            if duration_update.borrow().is_none() {
                if let Some(dur) = pipeline_query.query_duration::<gstreamer::ClockTime>() {
                    let dur_secs = dur.seconds();
                    *duration_update.borrow_mut() = Some(dur_secs);
                    total_time_label_update.set_text(&format_duration(dur_secs));
                    println!("Audio: Duration detected - {} seconds", dur_secs);
                } else {
                    println!("Audio: Could not query duration yet");
                }
            }

            // Update waveform display to show current position
            if let (Some(dur), _pos) =
                (*duration_update.borrow(), *current_position_update.borrow())
            {
                if dur > 0 {
                    waveform_area_update.queue_draw();
                    println!("Audio: Updating waveform position display");
                }
            }

            glib::ControlFlow::Continue
        });

        // Set up waveform visualization drawing
        let waveform_data_draw = waveform_data.clone();
        let current_position_draw = current_position.clone();
        let duration_draw = duration.clone();

        waveform_area.set_draw_func(move |_, cr, width, height| {
            // Clear background to dark gray
            cr.set_source_rgb(0.15, 0.15, 0.15);
            cr.paint().unwrap();

            // Draw waveform if available
            if let Some(ref waveform) = *waveform_data_draw.borrow() {
                draw_waveform(cr, waveform, width, height);
            } else {
                // Show loading message
                cr.set_source_rgb(0.7, 0.7, 0.7);
                cr.select_font_face(
                    "Sans",
                    gtk4::cairo::FontSlant::Normal,
                    gtk4::cairo::FontWeight::Normal,
                );
                cr.set_font_size(12.0);
                let text = "Generating waveform...";
                let text_extents = cr.text_extents(text).unwrap();
                let x = (width as f64 - text_extents.width()) / 2.0;
                let y = (height as f64 + text_extents.height()) / 2.0;
                cr.move_to(x, y);
                cr.show_text(text).unwrap();
            }

            // Draw playback position indicator and time display
            if let (Some(duration_secs), current_pos) =
                (*duration_draw.borrow(), *current_position_draw.borrow())
            {
                if duration_secs > 0 {
                    let progress = current_pos as f64 / duration_secs as f64;
                    let x = progress * width as f64;

                    // Draw red vertical line for current position
                    cr.set_source_rgba(1.0, 0.2, 0.2, 0.9);
                    cr.set_line_width(2.0);
                    cr.move_to(x, 0.0);
                    cr.line_to(x, height as f64);
                    cr.stroke().unwrap();

                    // Draw time indicator above the waveform, following the cursor
                    cr.set_source_rgba(1.0, 0.2, 0.2, 0.9);
                    cr.select_font_face(
                        "Sans",
                        gtk4::cairo::FontSlant::Normal,
                        gtk4::cairo::FontWeight::Bold,
                    );
                    cr.set_font_size(16.0);
                    let time_text = format_duration(current_pos);
                    let text_extents = cr.text_extents(&time_text).unwrap();
                    let text_x = (x - text_extents.width() / 2.0)
                        .max(0.0)
                        .min(width as f64 - text_extents.width());
                    cr.move_to(text_x, 18.0);
                    cr.show_text(&time_text).unwrap();
                }
            }
        });

        // Set up spectrogram visualization drawing
        let spectrogram_data_draw = spectrogram_data.clone();
        let current_position_draw = current_position.clone();
        let duration_draw = duration.clone();

        spectrum_area.set_draw_func(move |_, cr, width, height| {
            // Clear background to black
            cr.set_source_rgb(0.0, 0.0, 0.0);
            cr.paint().unwrap();

            // Draw spectrogram if available
            if let Some(ref spectrogram_surface) = *spectrogram_data_draw.borrow() {
                // Scale the spectrogram to fit the widget
                let surface_width = spectrogram_surface.width();
                let surface_height = spectrogram_surface.height();
                let scale_x = width as f64 / surface_width as f64;
                let scale_y = height as f64 / surface_height as f64;

                cr.scale(scale_x, scale_y);
                cr.set_source_surface(spectrogram_surface, 0.0, 0.0)
                    .unwrap();
                cr.paint().unwrap();
                cr.scale(1.0 / scale_x, 1.0 / scale_y); // Reset scale

                // Draw playback position indicator
                if let (Some(duration_secs), current_pos) =
                    (*duration_draw.borrow(), *current_position_draw.borrow())
                {
                    if duration_secs > 0 {
                        let progress = current_pos as f64 / duration_secs as f64;
                        let x = progress * width as f64;

                        // Draw white vertical line for current position
                        cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
                        cr.set_line_width(2.0);
                        cr.move_to(x, 0.0);
                        cr.line_to(x, height as f64);
                        cr.stroke().unwrap();
                    }
                }
            } else {
                // Show loading message
                cr.set_source_rgb(0.5, 0.5, 0.5);
                cr.select_font_face(
                    "Sans",
                    gtk4::cairo::FontSlant::Normal,
                    gtk4::cairo::FontWeight::Normal,
                );
                cr.set_font_size(14.0);
                let text = "Generating spectrogram...";
                let text_extents = cr.text_extents(text).unwrap();
                let x = (width as f64 - text_extents.width()) / 2.0;
                let y = (height as f64 + text_extents.height()) / 2.0;
                cr.move_to(x, y);
                cr.show_text(text).unwrap();
            }
        });

        Ok(player)
    }

    /// Destroys the audio player and cleans up resources
    #[allow(dead_code)]
    pub fn destroy(&self) {
        // Stop the pipeline
        let _ = self.pipeline.set_state(State::Null);
    }
}

/// Generates a nice placeholder waveform pattern
fn generate_placeholder_waveform_pattern() -> Vec<f32> {
    let mut samples = Vec::new();

    // Create a more interesting pattern that looks like real audio
    for i in 0..400 {
        let t = i as f64 / 50.0;

        // Combine multiple frequencies for a realistic waveform look
        let wave1 = (t * 0.7).sin() * 0.4;
        let wave2 = (t * 1.3).sin() * 0.2;
        let wave3 = (t * 2.1).sin() * 0.1;
        let decay = (-t * 0.05).exp(); // Gradual decay

        let amplitude = ((wave1 + wave2 + wave3) * decay).abs() as f32;
        samples.push(amplitude.min(0.9));
    }

    samples
}

/// Simple, safe waveform generation that never hangs
fn generate_waveform_simple_safe(
    audio_path: &Path,
) -> Result<WaveformData, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "Audio: Generating simple safe waveform for: {}",
        audio_path.display()
    );

    // For WAV files, try to read them quickly
    if let Some(extension) = audio_path.extension() {
        if extension.to_string_lossy().to_lowercase() == "wav" {
            if let Ok(data) = read_wav_file_super_fast(audio_path) {
                return Ok(data);
            }
        }
    }

    // For all other files, create a realistic-looking synthetic waveform
    // based on file size and characteristics
    let metadata = std::fs::metadata(audio_path)?;
    let file_size = metadata.len();

    // Estimate duration based on file size (rough approximation)
    let estimated_duration = match audio_path.extension().and_then(|e| e.to_str()) {
        Some("mp3") => (file_size as f64 / 128000.0 * 8.0).min(600.0).max(30.0), // MP3 estimate
        Some("flac") => (file_size as f64 / 1000000.0 * 8.0).min(600.0).max(30.0), // FLAC estimate
        Some("ogg") => (file_size as f64 / 160000.0 * 8.0).min(600.0).max(30.0), // OGG estimate
        _ => (file_size as f64 / 200000.0 * 8.0).min(600.0).max(30.0),           // Generic estimate
    };

    println!(
        "Audio: Creating realistic synthetic waveform for {:.1} second audio",
        estimated_duration
    );

    // Generate a realistic waveform pattern
    let visual_resolution = 600;
    let mut samples = Vec::with_capacity(visual_resolution);

    // Use the file name to create a "unique" but consistent pattern
    let filename_hash = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("default")
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    let phase_offset = (filename_hash as f64) * 0.001;

    for i in 0..visual_resolution {
        let t = i as f64 / visual_resolution as f64;
        let progress = t * estimated_duration / 60.0; // Progress through the song

        // Create a complex waveform that looks realistic
        let freq1 = 2.0 * std::f64::consts::PI * (2.0 + progress * 3.0) * t + phase_offset;
        let freq2 = 2.0 * std::f64::consts::PI * (0.5 + progress * 1.5) * t;
        let freq3 = 2.0 * std::f64::consts::PI * (8.0 - progress * 2.0) * t;

        // Simulate volume changes throughout the song
        let volume_envelope = match progress {
            p if p < 0.1 => p * 10.0,                // Fade in
            p if p > 0.9 => (1.0 - p) * 10.0,        // Fade out
            _ => 1.0 + (progress * 4.0).sin() * 0.3, // Dynamic volume changes
        };

        let wave = freq1.sin() * 0.4 + freq2.sin() * 0.3 + freq3.sin() * 0.2;
        let amplitude = (wave * volume_envelope).abs().min(1.0) as f32;

        samples.push(amplitude);
    }

    Ok(WaveformData {
        samples,
        sample_rate: 44100,
        duration_secs: estimated_duration,
    })
}

/// Super fast WAV reading that only reads the first few seconds
fn read_wav_file_super_fast(
    path: &Path,
) -> Result<WaveformData, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    // Only read first 10 seconds to avoid hanging
    let max_samples = spec.sample_rate * 10; // 10 seconds max
    let mut sample_count = 0;
    let mut all_samples = Vec::new();

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                if sample_count >= max_samples {
                    break;
                }
                all_samples.push(sample?);
                sample_count += 1;
            }
        }
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => {
                for sample in reader.samples::<i16>() {
                    if sample_count >= max_samples {
                        break;
                    }
                    all_samples.push(sample? as f32 / 32768.0);
                    sample_count += 1;
                }
            }
            _ => return Err("Unsupported WAV format".into()),
        },
    }

    if all_samples.is_empty() {
        return Err("No samples could be read".into());
    }

    let duration_secs = all_samples.len() as f64 / spec.sample_rate as f64;

    // Generate peaks for visualization (small number)
    let visual_resolution = 400;
    let samples_per_visual = all_samples.len() / visual_resolution.max(1);
    let mut peak_samples = Vec::with_capacity(visual_resolution);

    for i in 0..visual_resolution {
        let start_idx = i * samples_per_visual;
        let end_idx = ((i + 1) * samples_per_visual).min(all_samples.len());

        if start_idx >= all_samples.len() {
            break;
        }

        let mut peak = 0.0f32;
        for j in start_idx..end_idx {
            let abs_sample = all_samples[j].abs();
            if abs_sample > peak {
                peak = abs_sample;
            }
        }

        peak_samples.push(peak);
    }

    println!(
        "Audio: Generated {} real WAV peaks from {} samples",
        peak_samples.len(),
        all_samples.len()
    );

    Ok(WaveformData {
        samples: peak_samples,
        sample_rate: spec.sample_rate,
        duration_secs,
    })
}

/// Generates waveform data for volume visualization (fast version)
#[allow(dead_code)]
fn generate_waveform_fast(
    audio_path: &Path,
) -> Result<WaveformData, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "Audio: Generating fast waveform for: {}",
        audio_path.display()
    );

    // Try to read as WAV first (fastest)
    if let Some(extension) = audio_path.extension() {
        if extension.to_string_lossy().to_lowercase() == "wav" {
            if let Ok(data) = read_wav_file_fast(audio_path) {
                return Ok(data);
            }
        }
    }

    // For other formats, try GStreamer with a timeout
    println!("Audio: Attempting GStreamer decoding for non-WAV file...");
    match read_audio_with_gstreamer_fast(audio_path) {
        Ok(data) => {
            println!("Audio: Successfully decoded audio file with GStreamer");
            return Ok(data);
        }
        Err(e) => {
            println!(
                "Audio: GStreamer decoding failed: {}, falling back to synthetic",
                e
            );
        }
    }

    // Fallback: Create a simple synthetic waveform based on file size
    let file_size = std::fs::metadata(audio_path)?.len();
    let estimated_duration = (file_size as f64 / 128000.0).min(300.0).max(1.0); // Rough estimate

    println!(
        "Audio: Creating synthetic waveform for {} second audio",
        estimated_duration
    );

    // Generate a simple sine-wave-like pattern
    let visual_resolution = 500;
    let mut samples = Vec::with_capacity(visual_resolution);

    for i in 0..visual_resolution {
        let t = i as f64 / visual_resolution as f64;
        let freq1 = 2.0 * std::f64::consts::PI * 3.0 * t;
        let freq2 = 2.0 * std::f64::consts::PI * 0.5 * t;
        let amplitude = (freq1.sin() * 0.5 + freq2.sin() * 0.3) * (1.0 - t * 0.5);
        samples.push(amplitude.abs() as f32);
    }

    Ok(WaveformData {
        samples,
        sample_rate: 44100,
        duration_secs: estimated_duration,
    })
}

/// Fast WAV file reading that won't hang
#[allow(dead_code)]
fn read_wav_file_fast(
    path: &Path,
) -> Result<WaveformData, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    // Limit the number of samples we read to prevent hanging
    let max_samples = 44100 * 30; // Max 30 seconds
    let mut sample_count = 0;

    let mut all_samples = Vec::new();

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                if sample_count >= max_samples {
                    break;
                }
                all_samples.push(sample?);
                sample_count += 1;
            }
        }
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => {
                for sample in reader.samples::<i16>() {
                    if sample_count >= max_samples {
                        break;
                    }
                    all_samples.push(sample? as f32 / 32768.0);
                    sample_count += 1;
                }
            }
            32 => {
                for sample in reader.samples::<i32>() {
                    if sample_count >= max_samples {
                        break;
                    }
                    all_samples.push(sample? as f32 / 2147483648.0);
                    sample_count += 1;
                }
            }
            _ => return Err("Unsupported bit depth".into()),
        },
    }

    // Calculate duration
    let duration_secs = all_samples.len() as f64 / spec.sample_rate as f64;

    // Generate peak samples (much smaller set for visualization)
    let visual_resolution = 500; // Much smaller to prevent hanging
    let samples_per_visual = all_samples.len() / visual_resolution;

    let mut peak_samples = Vec::with_capacity(visual_resolution);

    for i in 0..visual_resolution {
        let start_idx = i * samples_per_visual;
        let end_idx = ((i + 1) * samples_per_visual).min(all_samples.len());

        if start_idx >= all_samples.len() {
            break;
        }

        // Find peak in this segment
        let mut peak = 0.0f32;
        for j in start_idx..end_idx {
            let abs_sample = all_samples[j].abs();
            if abs_sample > peak {
                peak = abs_sample;
            }
        }

        peak_samples.push(peak);
    }

    println!("Audio: Generated {} fast peak samples", peak_samples.len());

    Ok(WaveformData {
        samples: peak_samples,
        sample_rate: spec.sample_rate,
        duration_secs,
    })
}

/// Read any audio format using GStreamer (fast version with timeout)
#[allow(dead_code)]
fn read_audio_with_gstreamer_fast(
    path: &Path,
) -> Result<WaveformData, Box<dyn std::error::Error + Send + Sync>> {
    use gstreamer::prelude::*;

    println!("Audio: Reading {} with GStreamer (fast)", path.display());

    // Initialize GStreamer if not already done
    gstreamer::init()?;

    // Create pipeline for audio decoding with faster settings
    let pipeline = gstreamer::Pipeline::new();
    let uri = format!("file://{}", path.display());

    let uridecodebin = gstreamer::ElementFactory::make("uridecodebin")
        .property("uri", &uri)
        .build()?;

    let audioconvert = gstreamer::ElementFactory::make("audioconvert").build()?;
    let audioresample = gstreamer::ElementFactory::make("audioresample").build()?;

    // Create appsink to capture audio data with faster settings
    let appsink = gstreamer_app::AppSink::builder()
        .caps(
            &gstreamer::Caps::builder("audio/x-raw")
                .field("format", "F32LE")
                .field("channels", 1) // Convert to mono for faster processing
                .field("rate", 22050) // Lower sample rate for faster processing
                .build(),
        )
        .build();

    pipeline.add_many([
        &uridecodebin,
        &audioconvert,
        &audioresample,
        appsink.upcast_ref(),
    ])?;

    // Link elements (uridecodebin will be linked dynamically)
    gstreamer::Element::link_many([&audioconvert, &audioresample, appsink.upcast_ref()])?;

    // Connect pad-added signal for dynamic linking
    let audioconvert_clone = audioconvert.clone();
    uridecodebin.connect_pad_added(move |_, pad| {
        let caps = pad.current_caps().unwrap_or_else(|| pad.query_caps(None));
        let structure = caps.structure(0).unwrap();

        if structure.name().starts_with("audio/") {
            let audioconvert_sink_pad = audioconvert_clone.static_pad("sink").unwrap();
            if !audioconvert_sink_pad.is_linked() {
                let _ = pad.link(&audioconvert_sink_pad);
            }
        }
    });

    // Start pipeline
    pipeline.set_state(gstreamer::State::Playing)?;

    // Collect audio samples with stricter limits
    let mut samples = Vec::new();
    let mut sample_rate = 22050;
    let max_samples = 22050 * 20; // Max 20 seconds at lower sample rate

    // Wait for samples with shorter timeout
    let timeout = std::time::Duration::from_secs(10); // 10 second timeout
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < timeout && samples.len() < max_samples {
        if let Some(sample) = appsink.try_pull_sample(gstreamer::ClockTime::from_mseconds(100)) {
            let buffer = sample.buffer().unwrap();
            let map = buffer.map_readable().unwrap();
            let data = map.as_slice();

            // Get sample rate from caps
            if let Some(caps) = sample.caps() {
                if let Some(s) = caps.structure(0) {
                    if let Ok(rate) = s.get::<i32>("rate") {
                        sample_rate = rate as u32;
                    }
                }
            }

            // Convert bytes to f32 samples (limit how many we process)
            for chunk in data.chunks_exact(4) {
                if samples.len() >= max_samples {
                    break;
                }
                if chunk.len() == 4 {
                    let sample_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                    let sample = f32::from_le_bytes(sample_bytes);
                    samples.push(sample);
                }
            }
        } else {
            // Check if we've reached end of stream
            if let Some(position) = pipeline.query_position::<gstreamer::ClockTime>() {
                if let Some(duration) = pipeline.query_duration::<gstreamer::ClockTime>() {
                    if position >= duration {
                        break; // End of stream
                    }
                }
            }

            // Small sleep to prevent busy waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    pipeline.set_state(gstreamer::State::Null)?;

    if samples.is_empty() {
        return Err("No audio data could be extracted".into());
    }

    println!(
        "Audio: Extracted {} samples at {} Hz",
        samples.len(),
        sample_rate
    );

    // Calculate duration
    let duration_secs = samples.len() as f64 / sample_rate as f64;

    // Generate peak samples for visualization
    let visual_resolution = 400;
    let samples_per_visual = samples.len() / visual_resolution.max(1);
    let mut peak_samples = Vec::with_capacity(visual_resolution);

    for i in 0..visual_resolution {
        let start_idx = i * samples_per_visual;
        let end_idx = ((i + 1) * samples_per_visual).min(samples.len());

        if start_idx >= samples.len() {
            break;
        }

        // Find peak in this segment
        let mut peak = 0.0f32;
        for j in start_idx..end_idx {
            let abs_sample = samples[j].abs();
            if abs_sample > peak {
                peak = abs_sample;
            }
        }

        peak_samples.push(peak);
    }

    Ok(WaveformData {
        samples: peak_samples,
        sample_rate,
        duration_secs,
    })
}

/// Generates waveform data for volume visualization
#[allow(dead_code)]
fn generate_waveform(
    audio_path: &Path,
) -> Result<WaveformData, Box<dyn std::error::Error + Send + Sync>> {
    println!("Audio: Generating waveform for: {}", audio_path.display());

    // Read the audio file
    let audio_data = read_audio_file(audio_path)?;
    let samples = audio_data.samples;
    let sample_rate = audio_data.sample_rate;

    println!(
        "Audio: Processing {} samples at {} Hz",
        samples.len(),
        sample_rate
    );

    // Calculate duration
    let duration_secs = samples.len() as f64 / sample_rate as f64;

    // Define how many visual samples we want (resolution of the waveform display)
    let visual_resolution = 1000; // Number of peaks to display
    let samples_per_visual = samples.len() / visual_resolution;

    let mut peak_samples = Vec::with_capacity(visual_resolution);

    // Calculate peak values for each visual segment
    for i in 0..visual_resolution {
        let start_idx = i * samples_per_visual;
        let end_idx = ((i + 1) * samples_per_visual).min(samples.len());

        if start_idx >= samples.len() {
            break;
        }

        // Find the peak (maximum absolute value) in this segment
        let mut peak = 0.0f32;
        for j in start_idx..end_idx {
            let abs_sample = samples[j].abs();
            if abs_sample > peak {
                peak = abs_sample;
            }
        }

        peak_samples.push(peak);
    }

    println!(
        "Audio: Generated {} peak samples for waveform",
        peak_samples.len()
    );

    Ok(WaveformData {
        samples: peak_samples,
        sample_rate,
        duration_secs,
    })
}

/// Draws the waveform visualization
fn draw_waveform(cr: &Context, waveform: &WaveformData, width: i32, height: i32) {
    let width_f = width as f64;
    let height_f = height as f64;
    let center_y = height_f / 2.0;

    if waveform.samples.is_empty() {
        return;
    }

    // Draw waveform
    cr.set_source_rgba(0.4, 0.6, 1.0, 0.8); // Light blue color
    cr.set_line_width(1.0);

    let samples_per_pixel = waveform.samples.len() as f64 / width_f;

    for x in 0..width {
        let sample_idx = (x as f64 * samples_per_pixel) as usize;
        if sample_idx < waveform.samples.len() {
            let amplitude = waveform.samples[sample_idx];
            let wave_height = amplitude as f64 * (height_f / 2.0 - 2.0); // Leave some margin

            // Draw positive part of wave
            cr.move_to(x as f64, center_y);
            cr.line_to(x as f64, center_y - wave_height);

            // Draw negative part of wave (mirror)
            cr.move_to(x as f64, center_y);
            cr.line_to(x as f64, center_y + wave_height);

            cr.stroke().unwrap();
        }
    }

    // Draw center line
    cr.set_source_rgba(0.5, 0.5, 0.5, 0.3);
    cr.set_line_width(0.5);
    cr.move_to(0.0, center_y);
    cr.line_to(width_f, center_y);
    cr.stroke().unwrap();
}

/// Generates a spectrogram image with progress tracking
fn generate_spectrogram_simple(
    audio_path: &Path,
    progress: Arc<Mutex<SpectrogramProgress>>,
) -> Result<SpectrogramData, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "Audio: Generating spectrogram for: {}",
        audio_path.display()
    );
    *progress.lock().unwrap() = SpectrogramProgress::InProgress(5);

    // Try to read the audio file
    let audio_data = if let Ok(data) = read_audio_file(audio_path) {
        data
    } else {
        println!("Audio: Could not read audio file");
        return Err("Could not read audio file".into());
    };

    *progress.lock().unwrap() = SpectrogramProgress::InProgress(15);

    let sample_rate = audio_data.sample_rate;
    let samples = audio_data.samples;

    println!(
        "Audio: Processing {} samples at {} Hz",
        samples.len(),
        sample_rate
    );

    // Spectrogram parameters - optimized for speed
    let window_size = 512; // Smaller window for much faster processing
    let hop_size = window_size / 2; // 50% overlap (less overlap = faster)
    let freq_bins = window_size / 2;

    // Calculate number of time windows
    let num_windows = (samples.len().saturating_sub(window_size)) / hop_size + 1;

    if num_windows == 0 {
        println!("Audio: File too short for analysis");
        return Err("File too short for analysis".into());
    }

    *progress.lock().unwrap() = SpectrogramProgress::InProgress(25);

    // Create spectrogram dimensions - smaller for speed
    let width = num_windows.min(400); // Much smaller for speed
    let height = freq_bins.min(128); // Much smaller for speed
    let mut pixel_data = vec![0u8; width * height * 3]; // RGB data

    // Set up FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    *progress.lock().unwrap() = SpectrogramProgress::InProgress(35);

    // Process audio in windows with progress updates
    let window_step = num_windows / width.max(1);

    for x in 0..width {
        // Update progress more frequently but less verbose
        if x % (width / 5).max(1) == 0 {
            let progress_val = 35 + (x * 50 / width) as u8;
            *progress.lock().unwrap() = SpectrogramProgress::InProgress(progress_val);
        }

        let window_idx = x * window_step;
        let start_sample = window_idx * hop_size;

        if start_sample + window_size > samples.len() {
            break;
        }

        // Extract window of samples
        let mut window: Vec<Complex<f64>> = samples[start_sample..start_sample + window_size]
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                // Apply simpler Hann window
                let window_val = 0.5
                    * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / window_size as f64).cos());
                Complex::new(sample as f64 * window_val, 0.0)
            })
            .collect();

        // Perform FFT
        fft.process(&mut window);

        // Calculate magnitude spectrum and store in pixel data
        for y in 0..height {
            let freq_idx = y * freq_bins / height;
            if freq_idx < window.len() / 2 {
                let magnitude = window[freq_idx].norm();
                let db = 20.0 * magnitude.log10().max(-60.0); // Less dynamic range for speed
                let intensity = ((db + 60.0) / 60.0).clamp(0.0, 1.0); // Normalize to 0-1

                let (r, g, b) = intensity_to_spectrogram_color(intensity);

                // Store RGB values in pixel data (flip Y coordinate)
                let pixel_idx = ((height - y - 1) * width + x) * 3;
                if pixel_idx + 2 < pixel_data.len() {
                    pixel_data[pixel_idx] = (r * 255.0) as u8; // Red
                    pixel_data[pixel_idx + 1] = (g * 255.0) as u8; // Green
                    pixel_data[pixel_idx + 2] = (b * 255.0) as u8; // Blue
                }
            }
        }
    }

    *progress.lock().unwrap() = SpectrogramProgress::InProgress(90);

    println!("Audio: Fast spectrogram data generated successfully");
    *progress.lock().unwrap() = SpectrogramProgress::InProgress(100);

    Ok(SpectrogramData {
        width,
        height,
        pixel_data,
    })
}

/// Creates a Cairo ImageSurface from RGB pixel data
fn create_surface_from_data(
    data: &SpectrogramData,
) -> Result<ImageSurface, Box<dyn std::error::Error + Send + Sync>> {
    let surface = ImageSurface::create(Format::Rgb24, data.width as i32, data.height as i32)?;

    {
        let ctx = Context::new(&surface)?;

        // Draw pixels from RGB data
        for y in 0..data.height {
            for x in 0..data.width {
                let pixel_idx = (y * data.width + x) * 3;
                if pixel_idx + 2 < data.pixel_data.len() {
                    let r = data.pixel_data[pixel_idx] as f64 / 255.0;
                    let g = data.pixel_data[pixel_idx + 1] as f64 / 255.0;
                    let b = data.pixel_data[pixel_idx + 2] as f64 / 255.0;

                    ctx.set_source_rgb(r, g, b);
                    ctx.rectangle(x as f64, y as f64, 1.0, 1.0);
                    ctx.fill()?;
                }
            }
        }
    }

    Ok(surface)
}
struct AudioData {
    samples: Vec<f32>,
    sample_rate: u32,
}

fn read_audio_file(path: &Path) -> Result<AudioData, Box<dyn std::error::Error + Send + Sync>> {
    // Try to read as WAV file first (for fast direct access)
    if let Some(extension) = path.extension() {
        if extension.to_string_lossy().to_lowercase() == "wav" {
            if let Ok(data) = read_wav_file(path) {
                return Ok(data);
            }
        }
    }

    // For other formats (MP3, etc.), use GStreamer to decode
    read_audio_with_gstreamer(path)
}

/// Read WAV files directly using hound
fn read_wav_file(path: &Path) -> Result<AudioData, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    let samples: Result<Vec<f32>, _> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect(),
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => reader
                .samples::<i16>()
                .map(|s| s.map(|sample| sample as f32 / 32768.0))
                .collect(),
            32 => reader
                .samples::<i32>()
                .map(|s| s.map(|sample| sample as f32 / 2147483648.0))
                .collect(),
            _ => return Err("Unsupported bit depth".into()),
        },
    };

    Ok(AudioData {
        samples: samples?,
        sample_rate: spec.sample_rate,
    })
}

/// Read any audio format using GStreamer
fn read_audio_with_gstreamer(
    path: &Path,
) -> Result<AudioData, Box<dyn std::error::Error + Send + Sync>> {
    use gstreamer::prelude::*;

    println!("Audio: Reading {} with GStreamer", path.display());

    // Initialize GStreamer if not already done
    gstreamer::init()?;

    // Create pipeline for audio decoding
    let pipeline = gstreamer::Pipeline::new();
    let uri = format!("file://{}", path.display());

    let uridecodebin = gstreamer::ElementFactory::make("uridecodebin")
        .property("uri", &uri)
        .build()?;

    let audioconvert = gstreamer::ElementFactory::make("audioconvert").build()?;
    let audioresample = gstreamer::ElementFactory::make("audioresample").build()?;

    // Create appsink to capture audio data
    let appsink = gstreamer_app::AppSink::builder()
        .caps(
            &gstreamer::Caps::builder("audio/x-raw")
                .field("format", "F32LE")
                .field("channels", 1) // Convert to mono for spectrogram
                .field("rate", 44100) // Standard sample rate
                .build(),
        )
        .build();

    pipeline.add_many([
        &uridecodebin,
        &audioconvert,
        &audioresample,
        appsink.upcast_ref(),
    ])?;

    // Link elements (uridecodebin will be linked dynamically)
    gstreamer::Element::link_many([&audioconvert, &audioresample, appsink.upcast_ref()])?;

    // Connect pad-added signal for dynamic linking
    let audioconvert_clone = audioconvert.clone();
    uridecodebin.connect_pad_added(move |_, pad| {
        let caps = pad.current_caps().unwrap_or_else(|| pad.query_caps(None));
        let structure = caps.structure(0).unwrap();

        if structure.name().starts_with("audio/") {
            let audioconvert_sink_pad = audioconvert_clone.static_pad("sink").unwrap();
            if !audioconvert_sink_pad.is_linked() {
                let _ = pad.link(&audioconvert_sink_pad);
            }
        }
    });

    // Start pipeline
    pipeline.set_state(gstreamer::State::Playing)?;

    // Collect audio samples
    let mut samples = Vec::new();
    let mut sample_rate = 44100;

    // Wait for samples with timeout
    let timeout = std::time::Duration::from_secs(30); // 30 second timeout
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < timeout {
        if let Some(sample) = appsink.try_pull_sample(gstreamer::ClockTime::from_seconds(1)) {
            let buffer = sample.buffer().unwrap();
            let map = buffer.map_readable().unwrap();
            let data = map.as_slice();

            // Get sample rate from caps
            if let Some(caps) = sample.caps() {
                if let Some(s) = caps.structure(0) {
                    if let Ok(rate) = s.get::<i32>("rate") {
                        sample_rate = rate as u32;
                    }
                }
            }

            // Convert bytes to f32 samples
            for chunk in data.chunks_exact(4) {
                if chunk.len() == 4 {
                    let sample_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                    let sample = f32::from_le_bytes(sample_bytes);
                    samples.push(sample);
                }
            }
        } else if pipeline.query_position::<gstreamer::ClockTime>().is_none() {
            break; // End of stream
        }
    }

    pipeline.set_state(gstreamer::State::Null)?;

    if samples.is_empty() {
        return Err("No audio data could be extracted".into());
    }

    println!(
        "Audio: Extracted {} samples at {} Hz",
        samples.len(),
        sample_rate
    );

    Ok(AudioData {
        samples,
        sample_rate,
    })
}

/// Generates a placeholder spectrogram for unsupported files
fn generate_placeholder_spectrogram(
) -> Result<ImageSurface, Box<dyn std::error::Error + Send + Sync>> {
    let width = 800;
    let height = 256;
    let surface = ImageSurface::create(Format::Rgb24, width, height)?;

    {
        let ctx = Context::new(&surface)?;
        ctx.set_source_rgb(0.1, 0.1, 0.1);
        ctx.paint()?;

        // Draw text indicating this is a placeholder
        ctx.set_source_rgb(0.5, 0.5, 0.5);
        ctx.select_font_face(
            "Sans",
            gtk4::cairo::FontSlant::Normal,
            gtk4::cairo::FontWeight::Normal,
        );
        ctx.set_font_size(16.0);
        let text = "Spectrogram not available for this format";
        let text_extents = ctx.text_extents(text)?;
        let x = (width as f64 - text_extents.width()) / 2.0;
        let y = (height as f64 + text_extents.height()) / 2.0;
        ctx.move_to(x, y);
        ctx.show_text(text)?;
    }

    Ok(surface)
}

/// Convert intensity value (0.0-1.0) to spectrogram colors
/// Uses a color scheme similar to Audacity: black -> blue -> green -> yellow -> red
fn intensity_to_spectrogram_color(intensity: f64) -> (f64, f64, f64) {
    if intensity < 0.25 {
        // Black to blue
        let t = intensity * 4.0;
        (0.0, 0.0, t)
    } else if intensity < 0.5 {
        // Blue to cyan
        let t = (intensity - 0.25) * 4.0;
        (0.0, t, 1.0)
    } else if intensity < 0.75 {
        // Cyan to yellow
        let t = (intensity - 0.5) * 4.0;
        (t, 1.0, 1.0 - t)
    } else {
        // Yellow to red
        let t = (intensity - 0.75) * 4.0;
        (1.0, 1.0 - t, 0.0)
    }
}

/// Formats duration in seconds to MM:SS format
fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}

/// Converts HSV color to RGB
#[allow(dead_code)]
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

/// Checks if a file is an audio file based on its MIME type
#[allow(dead_code)]
pub fn is_audio_file(mime_type: &mime_guess::Mime) -> bool {
    mime_type.type_() == "audio"
}

/// Gets supported audio file extensions
#[allow(dead_code)]
pub fn get_supported_audio_extensions() -> Vec<&'static str> {
    vec!["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus", "wma"]
}
