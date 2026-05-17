    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_music_file() {
        assert!(is_music_file(Path::new("song.mp3")));
        assert!(is_music_file(Path::new("track.wav")));
        assert!(is_music_file(Path::new("audio.flac")));
        assert!(is_music_file(Path::new("music.ogg")));
        assert!(is_music_file(Path::new("sound.m4a")));
        
        assert!(!is_music_file(Path::new("document.txt")));
        assert!(!is_music_file(Path::new("image.png")));
        assert!(!is_music_file(Path::new("video.mp4")));
    }

    #[test]
    fn test_is_audio_file() {
        let audio_mime = mime_guess::from_path("test.mp3").first().unwrap();
        assert!(is_audio_file(&audio_mime));
        
        let text_mime = mime_guess::from_path("test.txt").first().unwrap();
        assert!(!is_audio_file(&text_mime));
    }

    #[test]
    fn test_get_supported_audio_extensions() {
        let extensions = get_supported_audio_extensions();
        assert!(extensions.contains(&"mp3"));
        assert!(extensions.contains(&"wav"));
        assert!(extensions.contains(&"flac"));
        assert!(extensions.contains(&"ogg"));
        assert!(extensions.len() >= 8);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0:00");
        assert_eq!(format_duration(30), "0:30");
        assert_eq!(format_duration(60), "1:00");
        assert_eq!(format_duration(90), "1:30");
        assert_eq!(format_duration(125), "2:05");
        assert_eq!(format_duration(3661), "61:01");
    }

    #[test]
    fn test_hsv_to_rgb() {
        // Test red (h=0)
        let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
        
        // Test white (no saturation)
        let (r, g, b) = hsv_to_rgb(0.0, 0.0, 1.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_global_volume_management() {
        let manager = GlobalVolumeManager::new();
        
        // Test initial volume
        let initial = manager.get_volume();
        assert!(initial >= 0.0 && initial <= 1.0);
        
        // Test setting volume
        manager.set_volume(0.5);
        assert!((manager.get_volume() - 0.5).abs() < 0.01);
        
        // Test clamping
        manager.set_volume(1.5);
        assert!((manager.get_volume() - 1.0).abs() < 0.01);
        
        manager.set_volume(-0.5);
        assert!((manager.get_volume() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_set_get_global_volume() {
        set_global_volume(0.75);
        let volume = get_global_volume();
        assert!((volume - 0.75).abs() < 0.01);
        
        // Reset to default
        set_global_volume(0.8);
    }

    #[test]
    fn test_waveform_data_creation() {
        let waveform = WaveformData {
            samples: vec![0.1, 0.5, 0.8, 0.3],
            sample_rate: 44100,
            duration_secs: 1.0,
        };
        
        assert_eq!(waveform.samples.len(), 4);
        assert_eq!(waveform.sample_rate, 44100);
    }

    #[test]
    fn test_intensity_to_spectrogram_color() {
        // Test low intensity (should be dark/black to blue)
        let (r, g, b) = intensity_to_spectrogram_color(0.0);
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
        
        // Test mid-low intensity (should have blue component)
        let (_r, _g, b) = intensity_to_spectrogram_color(0.2);
        assert!(b > 0.0);
        
        // Test high intensity (should be brighter - yellow/red)
        let (r2, _g2, _b2) = intensity_to_spectrogram_color(1.0);
        assert_eq!(r2, 1.0);
    }
