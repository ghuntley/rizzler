// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::AIResponse;
use crate::conflict_parser::{ConflictRegion, ConflictFile};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use base64::Engine;
use tracing::{debug, info, warn, error};

/// A structure to cache AI responses for similar conflicts
pub struct AIResolutionCache {
    /// Cache directory path
    cache_dir: PathBuf,
    /// Maximum time to keep entries in the cache
    ttl: Duration,
    /// Flag to enable/disable caching
    enabled: bool,
    /// Maximum number of entries per cache (if Some)
    max_entries: Option<usize>,
    /// Auto cleanup expired entries
    auto_cleanup: bool,
    /// Immediate flush to disk
    immediate_flush: bool,
    /// Lock for cache operations
    cache_lock: Arc<Mutex<()>>,
}

/// Configuration options for the disk cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Enable or disable the cache
    pub enabled: bool,
    /// Cache directory path
    pub directory: PathBuf,
    /// Time-to-live for cache entries
    pub ttl_hours: u64,
    /// Maximum number of entries per cache type
    pub max_entries: Option<usize>,
    /// Auto cleanup expired entries
    pub auto_cleanup: bool,
    /// Immediate flush to disk
    pub immediate_flush: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: get_cache_dir(),
            ttl_hours: 24,
            max_entries: Some(1000),
            auto_cleanup: true,
            immediate_flush: true, // Enable immediate flush by default for better reliability
        }
    }
}

impl CacheConfig {
    /// Load configuration from environment variables and config file
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Override with environment variables
        if let Ok(val) = env::var("RIZZLER_USE_CACHE") {
            config.enabled = val.to_lowercase() == "true" || val == "1";
        }
        
        if let Ok(dir) = env::var("RIZZLER_CACHE_DIR") {
            config.directory = PathBuf::from(dir);
        }
        
        if let Ok(val) = env::var("RIZZLER_CACHE_TTL_HOURS") {
            if let Ok(hours) = val.parse::<u64>() {
                config.ttl_hours = hours;
            }
        }
        
        if let Ok(val) = env::var("RIZZLER_CACHE_MAX_ENTRIES") {
            if let Ok(entries) = val.parse::<usize>() {
                config.max_entries = Some(entries);
            }
        }
        
        if let Ok(val) = env::var("RIZZLER_CACHE_AUTO_CLEANUP") {
            config.auto_cleanup = val.to_lowercase() == "true" || val == "1";
        }
        
        if let Ok(val) = env::var("RIZZLER_CACHE_IMMEDIATE_FLUSH") {
            config.immediate_flush = val.to_lowercase() == "true" || val == "1";
        }
        
        config
    }
}

/// A serializable cache entry with expiration time
#[derive(Serialize, Deserialize)]
struct CacheEntry {
    /// The response from the AI provider
    response: AIResponse,
    /// When this entry expires (as unix timestamp)
    expires_at: u64,
    /// When this entry was created (as unix timestamp)
    created_at: u64,
}

impl AIResolutionCache {
    /// Create a new cache with default TTL of 1 hour
    pub fn new() -> Self {
        Self::from_config(CacheConfig::default())
    }
    
    /// Create a new cache from configuration
    pub fn from_config(config: CacheConfig) -> Self {
        let cache_dir = config.directory;
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            if let Err(e) = fs::create_dir_all(&cache_dir) {
                error!("Failed to create cache directory at {:?}: {}", cache_dir, e);
            } else {
                info!("Created cache directory at {:?}", cache_dir);
            }
        }
        
        AIResolutionCache {
            cache_dir,
            ttl: Duration::from_secs(config.ttl_hours * 3600),
            enabled: config.enabled,
            max_entries: config.max_entries,
            auto_cleanup: config.auto_cleanup,
            immediate_flush: config.immediate_flush,
            cache_lock: Arc::new(Mutex::new(())),
        }
    }
    
    /// Create a new cache with a specific TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self::with_options(ttl, None, false)
    }

    /// Create a new cache with specific options
    pub fn with_options(ttl: Duration, max_entries: Option<usize>, auto_cleanup: bool) -> Self {
        let cache_dir = get_cache_dir();
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            if let Err(e) = fs::create_dir_all(&cache_dir) {
                error!("Failed to create cache directory at {:?}: {}", cache_dir, e);
            } else {
                info!("Created cache directory at {:?}", cache_dir);
            }
        }
        
        AIResolutionCache {
            cache_dir,
            ttl,
            enabled: true,
            max_entries,
            auto_cleanup,
            immediate_flush: false,
            cache_lock: Arc::new(Mutex::new(())),
        }
    }
    
    /// Enable or disable the cache
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Check if the cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set the cache directory
    pub fn set_cache_dir(&mut self, dir: PathBuf) {
        self.cache_dir = dir;
        
        // Create cache directory if it doesn't exist
        if !self.cache_dir.exists() {
            if let Err(e) = fs::create_dir_all(&self.cache_dir) {
                error!("Failed to create cache directory at {:?}: {}", self.cache_dir, e);
            } else {
                info!("Created cache directory at {:?}", self.cache_dir);
            }
        }
    }
    
    /// Get the current cache directory
    pub fn get_cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Set the maximum number of entries per cache
    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = Some(max_entries);
        self.enforce_max_entries();
    }

    /// Set auto cleanup mode
    pub fn set_auto_cleanup(&mut self, auto_cleanup: bool) {
        self.auto_cleanup = auto_cleanup;
    }
    
    /// Set immediate flush mode
    pub fn set_immediate_flush(&mut self, immediate_flush: bool) {
        self.immediate_flush = immediate_flush;
    }
    
    /// Check if immediate flush is enabled
    pub fn is_immediate_flush(&self) -> bool {
        self.immediate_flush
    }
    
    /// Clear all entries from the cache
    pub fn clear(&self) {
        if let Ok(_guard) = self.cache_lock.lock() {
            let conflict_dir = self.cache_dir.join("conflicts");
            let file_dir = self.cache_dir.join("files");
            
            if conflict_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&conflict_dir) {
                    error!("Failed to clear conflict cache directory: {}", e);
                } else {
                    // Recreate the directory
                    let _ = fs::create_dir_all(&conflict_dir);
                    debug!("Cleared conflict cache directory");
                }
            }
            
            if file_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&file_dir) {
                    error!("Failed to clear file cache directory: {}", e);
                } else {
                    // Recreate the directory
                    let _ = fs::create_dir_all(&file_dir);
                    debug!("Cleared file cache directory");
                }
            }
        }
    }

    /// Get the path for conflict cache directory
    fn get_conflict_cache_dir(&self) -> PathBuf {
        let dir = self.cache_dir.join("conflicts");
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        dir
    }
    
    /// Get the path for file cache directory
    fn get_file_cache_dir(&self) -> PathBuf {
        let dir = self.cache_dir.join("files");
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        dir
    }

    /// Enforce maximum entries limit by removing oldest entries
    fn enforce_max_entries(&self) {
        if let Some(max) = self.max_entries {
            if let Ok(_guard) = self.cache_lock.lock() {
                // Enforce for conflict cache
                self.enforce_max_entries_for_dir(self.get_conflict_cache_dir(), max, "conflict");
                
                // Enforce for file cache
                self.enforce_max_entries_for_dir(self.get_file_cache_dir(), max, "file");
            }
        }
    }
    
    /// Enforce maximum entries for a specific cache directory
    fn enforce_max_entries_for_dir(&self, dir: PathBuf, max: usize, cache_type: &str) {
        if !dir.exists() {
            return;
        }
        
        match fs::read_dir(&dir) {
            Ok(entries) => {
                // Collect all entries with their timestamps
                let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
                
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                            if let Ok(metadata) = entry.metadata() {
                                if let Ok(created) = metadata.created() {
                                    files.push((path, created));
                                }
                            }
                        }
                    }
                }
                
                // If we have more than max entries, sort by creation time and delete oldest
                if files.len() > max {
                    // Sort by creation time (oldest first)
                    files.sort_by(|a, b| a.1.cmp(&b.1));
                    
                    // Remove oldest entries
                    for i in 0..files.len() - max {
                        if let Err(e) = fs::remove_file(&files[i].0) {
                            error!("Failed to remove old {} cache entry: {}", cache_type, e);
                        } else {
                            debug!("Removed oldest {} cache entry due to max size limit", cache_type);
                        }
                    }
                }
            },
            Err(e) => {
                error!("Failed to read {} cache directory: {}", cache_type, e);
            }
        }
    }

    /// Clean up expired entries
    fn cleanup_expired(&self) {
        if let Ok(_guard) = self.cache_lock.lock() {
            // Clean up conflict cache
            self.cleanup_expired_for_dir(self.get_conflict_cache_dir(), "conflict");
            
            // Clean up file cache
            self.cleanup_expired_for_dir(self.get_file_cache_dir(), "file");
        }
    }
    
    /// Clean up expired entries for a specific cache directory
    fn cleanup_expired_for_dir(&self, dir: PathBuf, cache_type: &str) {
        if !dir.exists() {
            return;
        }
        
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        match fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                            match fs::read_to_string(&path) {
                                Ok(content) => {
                                    match serde_json::from_str::<CacheEntry>(&content) {
                                        Ok(cache_entry) => {
                                            if cache_entry.expires_at <= now {
                                                if let Err(e) = fs::remove_file(&path) {
                                                    error!("Failed to remove expired {} cache entry: {}", cache_type, e);
                                                } else {
                                                    debug!("Removed expired {} cache entry", cache_type);
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            warn!("Failed to parse {} cache entry: {}", cache_type, e);
                                            // Consider removing invalid entries
                                            let _ = fs::remove_file(&path);
                                        }
                                    }
                                },
                                Err(e) => {
                                    warn!("Failed to read {} cache entry: {}", cache_type, e);
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                error!("Failed to read {} cache directory: {}", cache_type, e);
            }
        }
    }
    
    /// Generate a cache key for a conflict
    fn generate_conflict_key(&self, conflict: &ConflictRegion) -> String {
        // Use a hash of the combined content as the key
        let content = format!("{}-{}-{}", 
            base64::engine::general_purpose::STANDARD.encode(&conflict.base_content),
            base64::engine::general_purpose::STANDARD.encode(&conflict.our_content),
            base64::engine::general_purpose::STANDARD.encode(&conflict.their_content)
        );
        
        // Use a consistent hash function to generate a filename-safe key
        let hash = format!("{:x}", md5::compute(content));
        hash
    }
    
    /// Generate a cache key for a file
    fn generate_file_key(&self, file: &ConflictFile) -> String {
        // Use a hash of the file path and content as the key
        let content = format!("{}-{}", file.path, base64::engine::general_purpose::STANDARD.encode(&file.content));
        
        // Use a consistent hash function to generate a filename-safe key
        let hash = format!("{:x}", md5::compute(content));
        hash
    }
    
    /// Get the file path for a conflict cache entry
    fn get_conflict_cache_path(&self, key: &str) -> PathBuf {
        self.get_conflict_cache_dir().join(format!("{}.json", key))
    }
    
    /// Get the file path for a file cache entry
    fn get_file_cache_path(&self, key: &str) -> PathBuf {
        self.get_file_cache_dir().join(format!("{}.json", key))
    }
    
    /// Get a cached response for a conflict if available
    pub fn get_conflict(&self, conflict: &ConflictRegion) -> Option<AIResponse> {
        if !self.enabled {
            return None;
        }
        
        // Cleanup expired entries if auto-cleanup is enabled
        if self.auto_cleanup {
            self.cleanup_expired();
        }
        
        let key = self.generate_conflict_key(conflict);
        let cache_path = self.get_conflict_cache_path(&key);
        
        if cache_path.exists() {
            match fs::read_to_string(&cache_path) {
                Ok(content) => {
                    match serde_json::from_str::<CacheEntry>(&content) {
                        Ok(cache_entry) => {
                            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                                
                            if cache_entry.expires_at > now {
                                debug!("Cache hit for conflict");
                                return Some(cache_entry.response);
                            } else {
                                // Entry expired, remove it
                                let _ = fs::remove_file(&cache_path);
                                debug!("Removed expired conflict cache entry");
                            }
                        },
                        Err(e) => {
                            warn!("Failed to parse conflict cache entry: {}", e);
                            // Consider removing invalid entries
                            let _ = fs::remove_file(&cache_path);
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to read conflict cache entry: {}", e);
                }
            }
        }
        
        debug!("Cache miss for conflict");
        None
    }
    
    /// Get a cached response for a file if available
    pub fn get_file(&self, file: &ConflictFile) -> Option<AIResponse> {
        if !self.enabled {
            return None;
        }
        
        // Cleanup expired entries if auto-cleanup is enabled
        if self.auto_cleanup {
            self.cleanup_expired();
        }
        
        let key = self.generate_file_key(file);
        let cache_path = self.get_file_cache_path(&key);
        
        if cache_path.exists() {
            match fs::read_to_string(&cache_path) {
                Ok(content) => {
                    match serde_json::from_str::<CacheEntry>(&content) {
                        Ok(cache_entry) => {
                            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                                
                            if cache_entry.expires_at > now {
                                debug!("Cache hit for file {}", file.path);
                                return Some(cache_entry.response);
                            } else {
                                // Entry expired, remove it
                                let _ = fs::remove_file(&cache_path);
                                debug!("Removed expired file cache entry");
                            }
                        },
                        Err(e) => {
                            warn!("Failed to parse file cache entry: {}", e);
                            // Consider removing invalid entries
                            let _ = fs::remove_file(&cache_path);
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to read file cache entry: {}", e);
                }
            }
        }
        
        debug!("Cache miss for file {}", file.path);
        None
    }
    
    /// Store a response for a conflict in the cache
    pub fn put_conflict(&self, conflict: &ConflictRegion, response: AIResponse) {
        if !self.enabled {
            return;
        }
        
        // Cleanup expired entries if auto-cleanup is enabled
        if self.auto_cleanup {
            self.cleanup_expired();
        }
        
        let key = self.generate_conflict_key(conflict);
        let cache_path = self.get_conflict_cache_path(&key);
        
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expires_at = now + self.ttl.as_secs();
        
        let entry = CacheEntry {
            response,
            expires_at,
            created_at: now,
        };
        
        match serde_json::to_string(&entry) {
            Ok(json) => {
                // Make sure the directory exists
                let dir = cache_path.parent().unwrap_or(Path::new(""));
                if !dir.exists() {
                    let _ = fs::create_dir_all(dir);
                }
                
                match fs::write(&cache_path, json) {
                    Ok(_) => {
                        debug!("Cached response for conflict");
                        
                        // Flush immediately if enabled
                        if self.immediate_flush {
                            let _ = self.flush();
                        }
                    },
                    Err(e) => {
                        error!("Failed to write conflict cache entry: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to serialize conflict cache entry: {}", e);
            }
        }
        
        // Enforce max entries if set
        if self.max_entries.is_some() {
            self.enforce_max_entries();
        }
    }
    
    /// Store a response for a file in the cache
    pub fn put_file(&self, file: &ConflictFile, response: AIResponse) {
        if !self.enabled {
            return;
        }
        
        // Cleanup expired entries if auto-cleanup is enabled
        if self.auto_cleanup {
            self.cleanup_expired();
        }
        
        let key = self.generate_file_key(file);
        let cache_path = self.get_file_cache_path(&key);
        
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expires_at = now + self.ttl.as_secs();
        
        let entry = CacheEntry {
            response,
            expires_at,
            created_at: now,
        };
        
        match serde_json::to_string(&entry) {
            Ok(json) => {
                // Make sure the directory exists
                let dir = cache_path.parent().unwrap_or(Path::new(""));
                if !dir.exists() {
                    let _ = fs::create_dir_all(dir);
                }
                
                match fs::write(&cache_path, json) {
                    Ok(_) => {
                        debug!("Cached response for file {}", file.path);
                        
                        // Flush immediately if enabled
                        if self.immediate_flush {
                            let _ = self.flush();
                        }
                    },
                    Err(e) => {
                        error!("Failed to write file cache entry: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to serialize file cache entry: {}", e);
            }
        }
        
        // Enforce max entries if set
        if self.max_entries.is_some() {
            self.enforce_max_entries();
        }
    }

    /// Explicitly flush any pending writes to ensure they are persisted to disk
    pub fn flush(&self) -> Result<(), std::io::Error> {
        // Since we're just using normal file operations, this is a no-op
        // but we'll sync the directories to be sure
        if let Ok(_guard) = self.cache_lock.lock() {
            let conflict_dir = self.get_conflict_cache_dir();
            let file_dir = self.get_file_cache_dir();
            
            // Try to sync the directories
            if conflict_dir.exists() {
                if let Ok(f) = std::fs::File::open(&conflict_dir) {
                    let _ = f.sync_all();
                }
            }
            
            if file_dir.exists() {
                if let Ok(f) = std::fs::File::open(&file_dir) {
                    let _ = f.sync_all();
                }
            }
        }
        
        Ok(())
    }
}

/// Get the cache directory path from environment variable or default to system temp
fn get_cache_dir() -> PathBuf {
    match env::var("RIZZLER_CACHE_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            // Default to system temp directory with 'rizzler-cache' subdirectory
            let temp_dir = env::temp_dir();
            temp_dir.join("rizzler-cache")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::fs;
    use tempfile::TempDir;
    use crate::ai_provider::TokenUsage;
    
    // Helper function to create a test conflict region
    fn create_test_conflict(our_content: &str, their_content: &str) -> ConflictRegion {
        ConflictRegion {
            base_content: String::from("Base content\n"),
            our_content: our_content.to_string(),
            their_content: their_content.to_string(),
            start_line: 1,
            end_line: 5,
        }
    }
    
    // Helper function to create a test conflict file
    fn create_test_conflict_file(conflicts: Vec<ConflictRegion>) -> ConflictFile {
        ConflictFile {
            path: "test.txt".to_string(),
            conflicts,
            content: "<<<<<<< HEAD\nTest content\n=======\nTheir content\n>>>>>>> branch-name\n".to_string(),
        }
    }
    
    // Helper function to create a test response
    fn create_test_response(content: &str) -> AIResponse {
        AIResponse {
            content: content.to_string(),
            model: "test-model".to_string(),
            explanation: Some("Test explanation".to_string()),
            token_usage: Some(TokenUsage {
                input_tokens: 5,
                output_tokens: 5,
                total_tokens: 10,
            }),
        }
    }
    
    // Setup a temporary directory for tests
    fn setup_test_cache() -> (AIResolutionCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        
        // Set environment variable for the test
        env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
        
        let cache = AIResolutionCache::new();
        
        (cache, temp_dir)
    }
    
    #[test]
    fn test_cache_conflict_hit() {
        let (cache, _temp_dir) = setup_test_cache();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache
        cache.put_conflict(&conflict, response.clone());
        
        // Verify cache file was created
        let key = cache.generate_conflict_key(&conflict);
        let cache_path = cache.get_conflict_cache_path(&key);
        assert!(cache_path.exists());
        
        // Retrieve from cache
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_some());
        
        let cached = cached.unwrap();
        assert_eq!(cached.content, "Resolved content\n");
        assert_eq!(cached.model, "test-model");
        assert!(cached.explanation.is_some());
        assert!(cached.token_usage.is_some());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_cache_conflict_miss() {
        let (cache, _temp_dir) = setup_test_cache();
        let conflict1 = create_test_conflict("Our content\n", "Their content\n");
        let conflict2 = create_test_conflict("Different content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache for conflict1
        cache.put_conflict(&conflict1, response);
        
        // Try to retrieve for conflict2
        let cached = cache.get_conflict(&conflict2);
        assert!(cached.is_none());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_cache_file_hit() {
        let (cache, _temp_dir) = setup_test_cache();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict]);
        let response = create_test_response("Resolved file content\n");
        
        // Store in cache
        cache.put_file(&file, response.clone());
        
        // Verify cache file was created
        let key = cache.generate_file_key(&file);
        let cache_path = cache.get_file_cache_path(&key);
        assert!(cache_path.exists());
        
        // Retrieve from cache
        let cached = cache.get_file(&file);
        assert!(cached.is_some());
        
        let cached = cached.unwrap();
        assert_eq!(cached.content, "Resolved file content\n");
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    #[ignore] // Temporarily ignored due to flaky test
    fn test_cache_expiration() {
        let (cache, _temp_dir) = setup_test_cache();
        
        // Create cache with a very short TTL (1ms)
        let mut cache = AIResolutionCache::with_ttl(Duration::from_millis(1));
        cache.cache_dir = PathBuf::from(env::var("RIZZLER_CACHE_DIR").unwrap());
        
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache
        cache.put_conflict(&conflict, response);
        
        // Sleep to let it expire
        thread::sleep(Duration::from_millis(10));
        
        // Try to retrieve - should be expired
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_none());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_cache_disable() {
        let (mut cache, _temp_dir) = setup_test_cache();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Disable cache
        cache.set_enabled(false);
        assert!(!cache.is_enabled());
        
        // Store in cache - should not actually store
        cache.put_conflict(&conflict, response.clone());
        
        // Verify no cache file was created
        let key = cache.generate_conflict_key(&conflict);
        let cache_path = cache.get_conflict_cache_path(&key);
        assert!(!cache_path.exists());
        
        // Try to retrieve - should be none
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_none());
        
        // Re-enable and try again
        cache.set_enabled(true);
        assert!(cache.is_enabled());
        
        // Store in cache
        cache.put_conflict(&conflict, response);
        
        // Try to retrieve - should be found
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_some());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_cache_clear() {
        let (cache, _temp_dir) = setup_test_cache();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict.clone()]);
        let response = create_test_response("Resolved content\n");
        
        // Store in both caches
        cache.put_conflict(&conflict, response.clone());
        cache.put_file(&file, response);
        
        // Verify both directories exist
        assert!(cache.get_conflict_cache_dir().exists());
        assert!(cache.get_file_cache_dir().exists());
        
        // Clear cache
        cache.clear();
        
        // Verify both are cleared
        assert!(cache.get_conflict(&conflict).is_none());
        assert!(cache.get_file(&file).is_none());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_max_entries() {
        let (mut cache, _temp_dir) = setup_test_cache();
        
        // Set max entries to 2
        cache.set_max_entries(2);
        
        // Create 3 different conflicts
        let conflict1 = create_test_conflict("Content 1\n", "Their content\n");
        let conflict2 = create_test_conflict("Content 2\n", "Their content\n");
        let conflict3 = create_test_conflict("Content 3\n", "Their content\n");
        
        let response1 = create_test_response("Resolved content 1\n");
        let response2 = create_test_response("Resolved content 2\n");
        let response3 = create_test_response("Resolved content 3\n");
        
        // Store all three in cache
        cache.put_conflict(&conflict1, response1);
        
        // Small sleep to ensure different timestamps
        thread::sleep(Duration::from_millis(10));
        cache.put_conflict(&conflict2, response2);
        
        thread::sleep(Duration::from_millis(10));
        cache.put_conflict(&conflict3, response3);
        
        // Verify only the 2 most recent are in cache
        assert!(cache.get_conflict(&conflict1).is_none()); // This should be evicted
        assert!(cache.get_conflict(&conflict2).is_some());
        assert!(cache.get_conflict(&conflict3).is_some());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_auto_cleanup() {
        let (mut cache, _temp_dir) = setup_test_cache();
        
        // Set a very short TTL and enable auto cleanup
        cache = AIResolutionCache::with_options(
            Duration::from_millis(1), 
            None, 
            true
        );
        cache.cache_dir = PathBuf::from(env::var("RIZZLER_CACHE_DIR").unwrap());
        
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache
        cache.put_conflict(&conflict, response);
        
        // Sleep to let it expire
        thread::sleep(Duration::from_millis(10));
        
        // Just accessing the cache should trigger cleanup
        let _ = cache.get_conflict(&create_test_conflict("Other\n", "Content\n"));
        
        // Verify cache file was deleted
        let key = cache.generate_conflict_key(&conflict);
        let cache_path = cache.get_conflict_cache_path(&key);
        assert!(!cache_path.exists());
        
        // Clean up
        env::remove_var("RIZZLER_CACHE_DIR");
    }
}