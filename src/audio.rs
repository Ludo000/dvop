// Audio playback functionality for the Basado Text Editor
// This module handles audio file playback using GStreamer

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Scale, Orientation, Image};
use glib::clone;
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

use gstreamer::prelude::*;
use gstreamer::{Pipeline, State, SeekFlags};

/// Audio player widget that provides playback controls and visualization
pub struct AudioPlayer {
    pub widget: GtkBox,
    pipeline: Pipeline,
    position_scale: Scale,
    play_button: Button,
    current_position: Rc<RefCell<u64>>,
    duration: Rc<RefCell<Option<u64>>>,
    is_playing: Rc<RefCell<bool>>,
}

impl AudioPlayer {
    /// Creates a new audio player widget for the given audio file
    pub fn new(audio_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Audio: Initializing GStreamer...");
        // Initialize GStreamer
        gstreamer::init().map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
        
        println!("Audio: Creating audio player for: {}", audio_path.display());
        
        // Create the main container
        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_top(20);
        main_box.set_margin_bottom(20);
        main_box.set_margin_start(20);
        main_box.set_margin_end(20);
        main_box.set_valign(gtk4::Align::Center);
        main_box.set_halign(gtk4::Align::Fill);
        
        // File info section
        let info_box = GtkBox::new(Orientation::Vertical, 8);
        
        // Audio file icon
        let audio_icon = Image::from_icon_name("audio-x-generic");
        audio_icon.set_pixel_size(64);
        audio_icon.set_margin_bottom(12);
        info_box.append(&audio_icon);
        
        // File name label
        let filename = audio_path.file_name()
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
        
        // Progress section
        let progress_box = GtkBox::new(Orientation::Vertical, 8);
        
        // Position scale (scrub bar)
        let position_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        position_scale.set_hexpand(true);
        position_scale.set_draw_value(false);
        position_scale.add_css_class("audio-progress");
        
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
        
        progress_box.append(&position_scale);
        progress_box.append(&time_box);
        main_box.append(&progress_box);
        
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
        
        // Create GStreamer pipeline using playbin
        let uri = format!("file://{}", audio_path.display());
        println!("Audio: Creating playbin with URI: {}", uri);
        
        let pipeline = gstreamer::ElementFactory::make("playbin")
            .property("uri", &uri)
            .build()
            .map_err(|e| -> Box<dyn std::error::Error> { 
                println!("Audio: Failed to create playbin: {:?}", e);
                Box::new(e) 
            })?
            .downcast::<Pipeline>()
            .map_err(|_| -> Box<dyn std::error::Error> { 
                println!("Audio: Failed to downcast to Pipeline");
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to create pipeline")) 
            })?;
            
        println!("Audio: Pipeline created successfully");
        
        // Try to set pipeline to PAUSED state to prepare it and get duration
        match pipeline.set_state(State::Paused) {
            Ok(_) => println!("Audio: Pipeline set to PAUSED state"),
            Err(e) => println!("Audio: Warning - could not set pipeline to PAUSED: {:?}", e),
        }
        
        // Set up message handling for debugging
        let bus = pipeline.bus().unwrap();
        let pipeline_debug = pipeline.clone();
        bus.add_watch(move |_, msg| {
            use gstreamer::MessageView;
            match msg.view() {
                MessageView::Error(err) => {
                    println!("Audio Error: {} ({})", err.error(), err.debug().unwrap_or_default());
                }
                MessageView::Warning(warn) => {
                    println!("Audio Warning: {} ({})", warn.error(), warn.debug().unwrap_or_default());
                }
                MessageView::Info(info) => {
                    println!("Audio Info: {} ({})", info.error(), info.debug().unwrap_or_default());
                }
                MessageView::StateChanged(state) => {
                    if msg.src() == Some(pipeline_debug.upcast_ref()) {
                        println!("Audio State changed from {:?} to {:?}", state.old(), state.current());
                    }
                }
                _ => {}
            }
            glib::ControlFlow::Continue
        }).expect("Failed to add bus watch");
        
        println!("Audio: Message handling set up");
        
        // State tracking
        let current_position = Rc::new(RefCell::new(0u64));
        let duration = Rc::new(RefCell::new(None));
        let is_playing = Rc::new(RefCell::new(false));
        let pending_seek_position = Rc::new(RefCell::new(None::<u64>));
        let is_seeking = Rc::new(RefCell::new(false)); // Create is_seeking here
        
        let player = AudioPlayer {
            widget: main_box,
            pipeline,
            position_scale: position_scale.clone(),
            play_button: play_button.clone(),
            current_position: current_position.clone(),
            duration: duration.clone(),
            is_playing: is_playing.clone(),
        };
        
        // Set up play/pause button handler
        let pipeline_play = player.pipeline.clone();
        let is_playing_play = is_playing.clone();
        let play_button_clone = play_button.clone();
        let pending_seek_play = pending_seek_position.clone();
        let position_scale_play = position_scale.clone();
        let current_position_play = current_position.clone();
        let duration_play = duration.clone();
        let is_seeking_play = is_seeking.clone();
        
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
                            let position_scale_delayed = position_scale_play.clone();
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
                                        println!("Audio: Delayed seek successful to {} seconds", seek_pos);
                                        *pending_seek_delayed.borrow_mut() = None; // Clear pending seek
                                        *current_position_delayed.borrow_mut() = seek_pos; // Update current position
                                        
                                        // Update slider to correct position immediately
                                        if let Some(dur) = *duration_delayed.borrow() {
                                            let progress = (seek_pos as f64 / dur as f64) * 100.0;
                                            position_scale_delayed.set_value(progress);
                                        }
                                        
                                        // Clear seeking flag after a short delay
                                        let is_seeking_reset = is_seeking_delayed.clone();
                                        glib::timeout_add_local_once(Duration::from_millis(200), move || {
                                            *is_seeking_reset.borrow_mut() = false;
                                        });
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
        let position_scale_stop = position_scale.clone();
        let current_time_label_stop = current_time_label.clone();
        let pending_seek_stop = pending_seek_position.clone();
        
        stop_button.connect_clicked(clone!(
            #[weak] pipeline_stop,
            #[weak] is_playing_stop,
            #[weak] play_button_stop,
            #[weak] position_scale_stop,
            #[weak] current_time_label_stop,
            #[weak] pending_seek_stop,
            move |_| {
                println!("Audio: Stop button clicked!");
                let _ = pipeline_stop.set_state(State::Paused); // Use PAUSED instead of READY to maintain duration info
                *is_playing_stop.borrow_mut() = false;
                *pending_seek_stop.borrow_mut() = None; // Clear any pending seek
                
                // Reset UI
                let stop_icon = Image::from_icon_name("media-playback-start");
                play_button_stop.set_child(Some(&stop_icon));
                play_button_stop.set_tooltip_text(Some("Play"));
                position_scale_stop.set_value(0.0);
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
        
        // Set up position scale (scrub bar) handler
        let pipeline_seek = player.pipeline.clone();
        let duration_seek = duration.clone();
        let is_seeking_clone = is_seeking.clone();
        let current_time_label_seek = current_time_label.clone();
        let is_playing_seek = is_playing.clone();
        let pending_seek_seek = pending_seek_position.clone();
        
        position_scale.connect_change_value(clone!(
            #[weak] pipeline_seek,
            #[weak] duration_seek,
            #[weak] is_seeking_clone,
            #[weak] current_time_label_seek,
            #[weak] is_playing_seek,
            #[weak] pending_seek_seek,
            #[upgrade_or] glib::Propagation::Proceed,
            move |_, _, value| {
                println!("Audio: Slider value changed to: {}", value);
                let duration_val = *duration_seek.borrow();
                println!("Audio: Current duration state: {:?}", duration_val);
                
                if let Some(dur) = duration_val {
                    *is_seeking_clone.borrow_mut() = true;
                    let seek_pos = (value as f64 / 100.0) * dur as f64;
                    let seek_pos_secs = seek_pos as u64;
                    println!("Audio: Seeking to position: {} seconds", seek_pos);
                    
                    // Update time label immediately
                    current_time_label_seek.set_text(&format_duration(seek_pos_secs));
                    
                    if *is_playing_seek.borrow() {
                        // Pipeline is playing/paused, seek immediately
                        let seek_time = gstreamer::ClockTime::from_seconds(seek_pos_secs);
                        match pipeline_seek.seek_simple(
                            gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                            seek_time,
                        ) {
                            Ok(_) => println!("Audio: Immediate seek successful"),
                            Err(e) => println!("Audio: Immediate seek failed: {:?}", e),
                        }
                    } else {
                        // Pipeline is stopped, store position for later
                        println!("Audio: Pipeline stopped, storing seek position {} seconds for later", seek_pos_secs);
                        *pending_seek_seek.borrow_mut() = Some(seek_pos_secs);
                        println!("Audio: Confirmed pending seek stored: {:?}", *pending_seek_seek.borrow());
                    }
                    
                    // Reset seeking flag after a delay
                    let is_seeking_reset = is_seeking_clone.clone();
                    glib::timeout_add_local_once(Duration::from_millis(200), move || {
                        *is_seeking_reset.borrow_mut() = false;
                    });
                } else {
                    println!("Audio: Duration not available yet, trying to query pipeline directly");
                    
                    // Try to query duration directly from pipeline
                    if let Some(dur) = pipeline_seek.query_duration::<gstreamer::ClockTime>() {
                        let dur_secs = dur.seconds();
                        *duration_seek.borrow_mut() = Some(dur_secs);
                        println!("Audio: Directly queried duration: {} seconds", dur_secs);
                        
                        // Now calculate and store the seek position
                        let seek_pos = (value as f64 / 100.0) * dur_secs as f64;
                        let seek_pos_secs = seek_pos as u64;
                        println!("Audio: Calculated seek position: {} seconds", seek_pos_secs);
                        
                        current_time_label_seek.set_text(&format_duration(seek_pos_secs));
                        
                        if !*is_playing_seek.borrow() {
                            println!("Audio: Storing seek position for later (direct query)");
                            *pending_seek_seek.borrow_mut() = Some(seek_pos_secs);
                            println!("Audio: Confirmed pending seek stored: {:?}", *pending_seek_seek.borrow());
                        }
                    } else {
                        println!("Audio: Cannot query duration yet, slider change ignored");
                    }
                }
                glib::Propagation::Proceed
            }
        ));
        
        // Set up periodic position updates with a simpler approach
        println!("Audio: Setting up position update timer...");
        
        let position_scale_update = position_scale.clone();
        let pipeline_query = player.pipeline.clone();
        let current_position_update = current_position.clone();
        let duration_update = duration.clone();
        let is_seeking_update = is_seeking.clone();
        let current_time_label_update = current_time_label.clone();
        let total_time_label_update = total_time_label.clone();
        let pending_seek_timer = pending_seek_position.clone();
        
        // Use a much simpler timeout approach
        let _timeout = glib::timeout_add_local(Duration::from_millis(1000), move || {
            println!("Audio: Timer callback executing...");
            
            if *is_seeking_update.borrow() {
                println!("Audio: Skipping update - currently seeking");
                return glib::ControlFlow::Continue;
            }
            
            // Don't update slider if there's a pending seek position (user has set slider manually)
            if pending_seek_timer.borrow().is_some() {
                println!("Audio: Skipping slider update - pending seek exists");
                // Still update duration and time labels, just not the slider position
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
            
            // Update progress bar
            if let (Some(dur), pos) = (*duration_update.borrow(), *current_position_update.borrow()) {
                if dur > 0 {
                    let progress = (pos as f64 / dur as f64) * 100.0;
                    position_scale_update.set_value(progress);
                    println!("Audio: Updating slider to {}%", progress);
                }
            }
            
            glib::ControlFlow::Continue
        });
        
        Ok(player)
    }
    
    /// Destroys the audio player and cleans up resources
    pub fn destroy(&self) {
        let _ = self.pipeline.set_state(State::Null);
    }
}

/// Formats duration in seconds to MM:SS format
fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}

/// Checks if a file is an audio file based on its MIME type
pub fn is_audio_file(mime_type: &mime_guess::Mime) -> bool {
    mime_type.type_() == "audio"
}

/// Gets supported audio file extensions
pub fn get_supported_audio_extensions() -> Vec<&'static str> {
    vec!["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus", "wma"]
}
