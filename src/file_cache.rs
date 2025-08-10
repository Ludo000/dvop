// File content caching system to optimize repeated file operations
// This module provides caching for file contents to avoid repeated disk I/O

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::fs;
use std::sync::{Arc, Mutex};

/// Represents a cached file entry with content and metadata
#[derive(Clone)]
struct CachedFile {
    content: String,
    last_modified: SystemTime,
    cached_at: SystemTime,
}

/// File content cache with automatic expiration and invalidation
pub struct FileCache {
    cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
    cache_duration: Duration,
}

impl FileCache {
    /// Create a new file cache with the specified cache duration
    pub fn new(cache_duration: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            cache_duration,
        }
    }
    
    /// Create a file cache with default 30-second duration
    pub fn with_default_duration() -> Self {
        Self::new(Duration::from_secs(30))
    }
    
    /// Get file content from cache or load from disk if not cached or expired
    pub fn get_file_content<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error> {
        let path = path.as_ref().to_path_buf();
        
        // Get file metadata
        let metadata = fs::metadata(&path)?;
        let file_modified_time = metadata.modified()?;
        
        let mut cache = self.cache.lock().unwrap();
        
        // Check if we have a valid cached entry
        if let Some(cached_file) = cache.get(&path) {
            let now = SystemTime::now();
            let cache_age = now.duration_since(cached_file.cached_at)
                .unwrap_or(Duration::from_secs(u64::MAX));
            
            // Return cached content if it's still valid and file hasn't been modified
            if cache_age < self.cache_duration && 
               cached_file.last_modified >= file_modified_time {
                return Ok(cached_file.content.clone());
            }
        }
        
        // Load file content from disk
        let content = fs::read_to_string(&path)?;
        
        // Cache the content
        let cached_file = CachedFile {
            content: content.clone(),
            last_modified: file_modified_time,
            cached_at: SystemTime::now(),
        };
        
        cache.insert(path, cached_file);
        Ok(content)
    }
    
    /// Clear the entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
    
    /// Remove a specific file from the cache
    pub fn invalidate<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref().to_path_buf();
        let mut cache = self.cache.lock().unwrap();
        cache.remove(&path);
    }
    
    /// Get the number of cached files
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Remove expired entries from the cache
    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.lock().unwrap();
        let now = SystemTime::now();
        
        cache.retain(|_, cached_file| {
            let cache_age = now.duration_since(cached_file.cached_at)
                .unwrap_or(Duration::from_secs(u64::MAX));
            cache_age < self.cache_duration
        });
    }
}

// Global file cache instance
lazy_static::lazy_static! {
    static ref GLOBAL_FILE_CACHE: FileCache = FileCache::with_default_duration();
}

/// Get cached file content using the global cache
pub fn get_cached_file_content(path: &std::path::Path) -> std::io::Result<String> {
    GLOBAL_FILE_CACHE.get_file_content(path)
}

/// Clear the global file cache
pub fn clear_file_cache() {
    GLOBAL_FILE_CACHE.clear();
}

/// Invalidate a specific file in the global cache
pub fn invalidate_file_cache<P: AsRef<Path>>(path: P) {
    GLOBAL_FILE_CACHE.invalidate(path);
}

/// Clean up expired entries in the global cache
pub fn cleanup_file_cache() {
    GLOBAL_FILE_CACHE.cleanup_expired();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_cache_basic() {
        let cache = FileCache::with_default_duration();
        
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
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
}
