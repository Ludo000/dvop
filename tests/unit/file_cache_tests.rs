    use super::*;
    use std::io::Write;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_cache_basic() {
        let cache = FileCache::with_default_duration();

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        writeln!(temp_file, "Hello, World!").unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path();

        // First read should load from disk
        let content1 = cache.get_file_content(path).unwrap();
        assert_eq!(content1, "Hello, World!\n");

        // Second read should come from cache
        let content2 = cache.get_file_content(path).unwrap();
        assert_eq!(content2, "Hello, World!\n");

        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = FileCache::with_default_duration();

        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write initial content
        std::fs::write(&file_path, "Original content\n").unwrap();

        // Load initial content
        let content1 = cache.get_file_content(&file_path).unwrap();
        assert_eq!(content1, "Original content\n");

        // Invalidate cache
        cache.invalidate(&file_path);

        // Modify file
        std::fs::write(&file_path, "Updated content\n").unwrap();

        // Should read updated content
        let content2 = cache.get_file_content(&file_path).unwrap();
        assert_eq!(content2, "Updated content\n");
    }

    #[test]
    fn test_cache_expiration_reloads_file() {
        let cache = FileCache::new(std::time::Duration::from_secs(0));
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("expires.txt");

        std::fs::write(&file_path, "first").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "first");

        std::fs::write(&file_path, "second").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "second");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cleanup_expired_removes_stale_entries() {
        let cache = FileCache::new(std::time::Duration::from_secs(0));
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("cleanup.txt");

        std::fs::write(&file_path, "cached").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "cached");
        assert_eq!(cache.len(), 1);

        cache.cleanup_expired();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_missing_file_returns_error_without_cache_entry() {
        let cache = FileCache::with_default_duration();
        let temp_dir = tempfile::tempdir().unwrap();
        let missing_path = temp_dir.path().join("missing.txt");

        let err = cache.get_file_content(&missing_path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn global_cache_reads_and_invalidates_entries() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("global-cache.txt");

        std::fs::write(&file_path, "version-one").unwrap();
        assert_eq!(get_cached_file_content(&file_path).unwrap(), "version-one");

        std::fs::write(&file_path, "version-two").unwrap();
        invalidate_file_cache(&file_path);
        assert_eq!(get_cached_file_content(&file_path).unwrap(), "version-two");
    }

    #[test]
    fn global_cache_tracks_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("one.txt");
        let second = dir.path().join("two.txt");

        std::fs::write(&first, "one").unwrap();
        std::fs::write(&second, "two").unwrap();

        assert_eq!(get_cached_file_content(&first).unwrap(), "one");
        assert_eq!(get_cached_file_content(&second).unwrap(), "two");
    }

    #[test]
    fn cleanup_file_cache_is_safe_after_populating_global_cache() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("cleanup-safe.txt");

        std::fs::write(&file_path, "content").unwrap();
        assert_eq!(get_cached_file_content(&file_path).unwrap(), "content");

        cleanup_file_cache();

        assert_eq!(get_cached_file_content(&file_path).unwrap(), "content");
    }

    #[test]
    fn file_cache_tracks_multiple_distinct_paths() {
        let cache = FileCache::with_default_duration();
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("one.txt");
        let second = dir.path().join("two.txt");
        std::fs::write(&first, "one").unwrap();
        std::fs::write(&second, "two").unwrap();

        assert_eq!(cache.get_file_content(&first).unwrap(), "one");
        assert_eq!(cache.get_file_content(&second).unwrap(), "two");
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn invalidate_removes_cached_entry_before_reload() {
        let cache = FileCache::with_default_duration();
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("invalidate.txt");

        std::fs::write(&file_path, "first").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "first");

        cache.invalidate(&file_path);
        std::fs::write(&file_path, "second").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "second");
    }

    #[test]
    fn get_file_content_reload_detects_on_disk_changes_without_invalidate() {
        let cache = FileCache::new(Duration::from_secs(60));
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("mtime.txt");

        std::fs::write(&file_path, "first").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "first");

        std::thread::sleep(Duration::from_millis(10));
        std::fs::write(&file_path, "second").unwrap();
        assert_eq!(cache.get_file_content(&file_path).unwrap(), "second");
    }

    #[test]
    fn file_cache_len_tracks_number_of_cached_entries() {
        let cache = FileCache::new(Duration::from_secs(60));
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("one.txt");
        let second = dir.path().join("two.txt");
        std::fs::write(&first, "one").unwrap();
        std::fs::write(&second, "two").unwrap();

        assert_eq!(cache.len(), 0);
        cache.get_file_content(&first).unwrap();
        assert_eq!(cache.len(), 1);
        cache.get_file_content(&second).unwrap();
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn get_file_content_returns_error_for_missing_file() {
        let cache = FileCache::with_default_duration();
        let missing = std::path::PathBuf::from("/tmp/dvop-file-cache-missing.txt");
        assert!(cache.get_file_content(&missing).is_err());
    }
