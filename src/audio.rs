// Audio playback functionality for the Basado Text Editor
// This module handles audio file playback using GStreamer

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Scale, Orientation, Image, DrawingArea, MenuButton, Popover};
use glib::clone;
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use gtk4::cairo::{ImageSurface, Format, Context};
use rustfft::{FftPlanner, num_complex::Complex};
use hound;

use gstreamer::prelude::*;
use gstreamer::{Pipeline, State, SeekFlags, Element};

/// Progress tracking for spectrogram generation
#[derive(Debug, Clone)]
enum SpectrogramProgress {
    NotStarted,
    InProgress(u8),       // Progress percentage
    Complete(SpectrogramData), // Completed spectrogram data
    Error(String),        // Error message
}

/// Thread-safe spectrogram data that can be sent between threads
#[derive(Debug, Clone)]
struct SpectrogramData {
    width: usize,
    height: usize,
    pixel_data: Vec<u8>, // RGB pixel data
}

/// Audio player widget that provides playback controls and visualization
pub struct AudioPlayer {
    pub widget: GtkBox,
    pipeline: Pipeline,
    position_scale: Scale,
    play_button: Button,
    current_position: Rc<RefCell<u64>>,
    duration: Rc<RefCell<Option<u64>>>,
    is_playing: Rc<RefCell<bool>>,
    spectrogram_data: Rc<RefCell<Option<ImageSurface>>>,
    spectrum_area: DrawingArea,
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
        
        // Spectrogram data (will be generated on button click)
        let spectrogram_data = Rc::new(RefCell::new(None));
        let audio_path_for_spectrogram = audio_path.to_path_buf();
        
        // Set up simple message handling
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
        
        let player = AudioPlayer {
            widget: main_box,
            pipeline,
            position_scale: position_scale.clone(),
            play_button: play_button.clone(),
            current_position: current_position.clone(),
            duration: duration.clone(),
            is_playing: is_playing.clone(),
            spectrogram_data: spectrogram_data.clone(),
            spectrum_area: spectrum_area.clone(),
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
                        *progress_data_thread.lock().unwrap() = SpectrogramProgress::Complete(spectrogram);
                    }
                    Err(e) => {
                        println!("Audio: Spectrogram generation failed: {}", e);
                        *progress_data_thread.lock().unwrap() = SpectrogramProgress::Error(format!("{}", e));
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
                cr.set_source_surface(spectrogram_surface, 0.0, 0.0).unwrap();
                cr.paint().unwrap();
                cr.scale(1.0/scale_x, 1.0/scale_y); // Reset scale
                
                // Draw playback position indicator
                if let (Some(duration_secs), current_pos) = (*duration_draw.borrow(), *current_position_draw.borrow()) {
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
                cr.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Normal);
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
    pub fn destroy(&self) {
        let _ = self.pipeline.set_state(State::Null);
    }
}

/// Generates a spectrogram image with progress tracking
fn generate_spectrogram_simple(audio_path: &Path, progress: Arc<Mutex<SpectrogramProgress>>) -> Result<SpectrogramData, Box<dyn std::error::Error + Send + Sync>> {
    println!("Audio: Generating spectrogram for: {}", audio_path.display());
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
    
    println!("Audio: Processing {} samples at {} Hz", samples.len(), sample_rate);
    
    // Spectrogram parameters - optimized for speed
    let window_size = 512;  // Smaller window for much faster processing
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
    let height = freq_bins.min(128);   // Much smaller for speed
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
                let window_val = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / window_size as f64).cos());
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
                    pixel_data[pixel_idx] = (r * 255.0) as u8;     // Red
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
fn create_surface_from_data(data: &SpectrogramData) -> Result<ImageSurface, Box<dyn std::error::Error + Send + Sync>> {
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
        hound::SampleFormat::Float => {
            reader.samples::<f32>().collect()
        }
        hound::SampleFormat::Int => {
            match spec.bits_per_sample {
                16 => {
                    reader.samples::<i16>()
                        .map(|s| s.map(|sample| sample as f32 / 32768.0))
                        .collect()
                }
                32 => {
                    reader.samples::<i32>()
                        .map(|s| s.map(|sample| sample as f32 / 2147483648.0))
                        .collect()
                }
                _ => return Err("Unsupported bit depth".into()),
            }
        }
    };
    
    Ok(AudioData {
        samples: samples?,
        sample_rate: spec.sample_rate,
    })
}

/// Read any audio format using GStreamer
fn read_audio_with_gstreamer(path: &Path) -> Result<AudioData, Box<dyn std::error::Error + Send + Sync>> {
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
        .caps(&gstreamer::Caps::builder("audio/x-raw")
            .field("format", "F32LE")
            .field("channels", 1) // Convert to mono for spectrogram
            .field("rate", 44100) // Standard sample rate
            .build())
        .build();
    
    pipeline.add_many(&[&uridecodebin, &audioconvert, &audioresample, appsink.upcast_ref()])?;
    
    // Link elements (uridecodebin will be linked dynamically)
    gstreamer::Element::link_many(&[&audioconvert, &audioresample, appsink.upcast_ref()])?;
    
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
    
    println!("Audio: Extracted {} samples at {} Hz", samples.len(), sample_rate);
    
    Ok(AudioData {
        samples,
        sample_rate,
    })
}

/// Generates a placeholder spectrogram for unsupported files
fn generate_placeholder_spectrogram() -> Result<ImageSurface, Box<dyn std::error::Error + Send + Sync>> {
    let width = 800;
    let height = 256;
    let surface = ImageSurface::create(Format::Rgb24, width, height)?;
    
    {
        let ctx = Context::new(&surface)?;
        ctx.set_source_rgb(0.1, 0.1, 0.1);
        ctx.paint()?;
        
        // Draw text indicating this is a placeholder
        ctx.set_source_rgb(0.5, 0.5, 0.5);
        ctx.select_font_face("Sans", gtk4::cairo::FontSlant::Normal, gtk4::cairo::FontWeight::Normal);
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
pub fn is_audio_file(mime_type: &mime_guess::Mime) -> bool {
    mime_type.type_() == "audio"
}

/// Gets supported audio file extensions
pub fn get_supported_audio_extensions() -> Vec<&'static str> {
    vec!["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus", "wma"]
}
