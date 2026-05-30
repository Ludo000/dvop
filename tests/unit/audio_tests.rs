    use super::*;
    use std::path::Path;

    fn write_test_wav(path: &Path, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
    }

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
    fn test_is_music_file_is_case_insensitive() {
        assert!(is_music_file(Path::new("TRACK.MP3")));
        assert!(is_music_file(Path::new("Song.WAV")));
    }

    #[test]
    fn test_is_music_content_without_extension_uses_path_heuristics() {
        assert!(is_music_content(Path::new("/home/user/Music/Artist - Title")));
        assert!(is_music_content(Path::new("/home/user/library/1 - Intro")));
        assert!(is_music_content(Path::new("/tmp/acoustic-track")));

        assert!(!is_music_content(Path::new("/tmp/podcast/interview")));
        assert!(!is_music_content(Path::new("/tmp/voice_memo")));
        assert!(!is_music_content(Path::new("/tmp/plainfile")));
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
    fn test_placeholder_waveform_pattern_is_bounded_and_stable_length() {
        let samples = generate_placeholder_waveform_pattern();

        assert_eq!(samples.len(), 400);
        assert!(samples.iter().all(|sample| *sample >= 0.0 && *sample <= 0.9));
        assert!(samples.iter().any(|sample| *sample > 0.0));
    }

    #[test]
    fn test_generate_waveform_simple_safe_creates_synthetic_waveform_for_non_wav() {
        let temp_dir = tempfile::tempdir().unwrap();
        let audio_path = temp_dir.path().join("song.mp3");
        std::fs::write(&audio_path, vec![1u8; 4096]).unwrap();

        let waveform = generate_waveform_simple_safe(&audio_path).unwrap();

        assert_eq!(waveform.samples.len(), 600);
        assert_eq!(waveform.sample_rate, 44100);
        assert!(waveform.duration_secs >= 30.0);
        assert!(waveform.samples.iter().all(|sample| *sample >= 0.0 && *sample <= 1.0));
    }

    #[test]
    fn test_read_wav_file_super_fast_reads_peaks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wav_path = temp_dir.path().join("peaks.wav");
        let samples: Vec<i16> = (0..800)
            .map(|i| if i % 2 == 0 { i16::MAX } else { i16::MIN + 1 })
            .collect();
        write_test_wav(&wav_path, &samples);

        let waveform = read_wav_file_super_fast(&wav_path).unwrap();

        assert_eq!(waveform.sample_rate, 8_000);
        assert!(!waveform.samples.is_empty());
        assert!(waveform.samples.iter().any(|sample| *sample > 0.9));
        assert!(waveform.duration_secs > 0.0);
    }

    #[test]
    fn test_read_wav_file_supports_i16_samples() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wav_path = temp_dir.path().join("audio.wav");
        write_test_wav(&wav_path, &[0, i16::MAX, i16::MIN + 1]);

        let audio = read_wav_file(&wav_path).unwrap();

        assert_eq!(audio.sample_rate, 8_000);
        assert_eq!(audio.samples.len(), 3);
        assert!(audio.samples[0].abs() < 0.001);
        assert!(audio.samples[1] > 0.99);
        assert!(audio.samples[2] < -0.99);
    }

    #[test]
    fn test_read_audio_file_prefers_wav_reader() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wav_path = temp_dir.path().join("audio.wav");
        write_test_wav(&wav_path, &[0, 1024, -1024]);

        let audio = read_audio_file(&wav_path).unwrap();

        assert_eq!(audio.sample_rate, 8_000);
        assert_eq!(audio.samples.len(), 3);
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

    #[test]
    fn test_intensity_to_spectrogram_color_boundaries() {
        assert_eq!(intensity_to_spectrogram_color(0.25), (0.0, 0.0, 1.0));
        assert_eq!(intensity_to_spectrogram_color(0.5), (0.0, 1.0, 1.0));
        assert_eq!(intensity_to_spectrogram_color(0.75), (1.0, 1.0, 0.0));
    }

    #[test]
    fn test_create_surface_from_spectrogram_data() {
        let data = SpectrogramData {
            width: 2,
            height: 2,
            pixel_data: vec![
                255, 0, 0,
                0, 255, 0,
                0, 0, 255,
                255, 255, 255,
            ],
        };

        let surface = create_surface_from_data(&data).unwrap();

        assert_eq!(surface.width(), 2);
        assert_eq!(surface.height(), 2);
    }

    #[test]
    fn test_generate_placeholder_spectrogram_has_expected_size() {
        let surface = generate_placeholder_spectrogram().unwrap();

        assert_eq!(surface.width(), 800);
        assert_eq!(surface.height(), 256);
    }

    #[test]
    fn test_generate_spectrogram_simple_updates_progress_and_returns_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wav_path = temp_dir.path().join("spectrum.wav");
        let samples: Vec<i16> = (0..1024)
            .map(|i| ((i as f32 / 1024.0 * std::f32::consts::TAU).sin() * i16::MAX as f32) as i16)
            .collect();
        write_test_wav(&wav_path, &samples);
        let progress = Arc::new(Mutex::new(SpectrogramProgress::NotStarted));

        let spectrogram = generate_spectrogram_simple(&wav_path, progress.clone()).unwrap();

        assert!(spectrogram.width > 0);
        assert!(spectrogram.height > 0);
        assert_eq!(spectrogram.pixel_data.len(), spectrogram.width * spectrogram.height * 3);
        assert!(matches!(
            *progress.lock().unwrap(),
            SpectrogramProgress::InProgress(100)
        ));
    }

    #[test]
    fn test_stop_audio_for_file_without_registered_players() {
        stop_audio_for_file(Path::new("/tmp/not-playing.mp3"));
    }

    #[test]
    fn test_stop_all_audio_players_without_registered_players() {
        stop_all_audio_players();
    }
