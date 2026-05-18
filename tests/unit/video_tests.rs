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
    fn test_is_video_file_supports_all_extensions_case_insensitive() {
        for file_name in [
            "movie.MP4",
            "clip.m4v",
            "archive.wmv",
            "camera.mpg",
            "camera.mpeg",
            "phone.3gp",
            "open.ogv",
        ] {
            assert!(is_video_file(Path::new(file_name)), "{} should be video", file_name);
        }

        assert!(!is_video_file(Path::new("video")));
        assert!(!is_video_file(Path::new("video.mp4.backup")));
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
    fn test_video_manager_registers_and_cleans_null_players() {
        gstreamer::init().ok();
        let manager = GlobalVideoManager::new();
        let pipeline = gstreamer::Pipeline::new();

        manager.register_player(&pipeline, "player_null".to_string());
        assert_eq!(manager.active_players.lock().unwrap().len(), 1);

        manager.cleanup_dead_players();
        assert_eq!(manager.active_players.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_video_manager_stop_players_for_file_removes_matching_player() {
        gstreamer::init().ok();
        let manager = GlobalVideoManager::new();
        let pipeline = gstreamer::Pipeline::new();
        let _ = pipeline.set_state(gstreamer::State::Ready);

        manager.register_player(&pipeline, "player_123_movie.mp4".to_string());
        manager.stop_players_for_file(Path::new("/tmp/movie.mp4"));

        assert_eq!(manager.active_players.lock().unwrap().len(), 0);
        assert!(manager.check_and_clear_stop_notification("player_123_movie.mp4"));

        let _ = pipeline.set_state(gstreamer::State::Null);
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
