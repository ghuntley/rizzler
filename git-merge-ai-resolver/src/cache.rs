// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIResponse, TokenUsage};
use crate::conflict_parser::{ConflictRegion, ConflictFile};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

/// A structure to cache AI responses for similar conflicts
pub struct AIResolutionCache {
    /// Cache for individual conflict resolutions
    conflict_cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    /// Cache for whole file resolutions
    file_cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    /// Maximum time to keep entries in the cache
    ttl: Duration,
    /// Flag to enable/disable caching
    enabled: bool,
    /// Maximum number of entries per cache (if Some)
    max_entries: Option<usize>,
    /// Auto cleanup expired entries
    auto_cleanup: bool,
    /// Access order for conflict cache (newest to oldest)
    conflict_access_order: Arc<Mutex<VecDeque<String>>>,
    /// Access order for file cache (newest to oldest)
    file_access_order: Arc<Mutex<VecDeque<String>>>,
}

/// A cache entry with expiration time
struct CacheEntry {
    /// The response from the AI provider
    response: AIResponse,
    /// When this entry expires
    expires_at: SystemTime,
}

impl AIResolutionCache {
    /// Create a new cache with default TTL of 1 hour
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(3600))
    }
    
    /// Create a new cache with a specific TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self::with_options(ttl, None, false)
    }

    /// Create a new cache with specific options
    pub fn with_options(ttl: Duration, max_entries: Option<usize>, auto_cleanup: bool) -> Self {
        AIResolutionCache {
            conflict_cache: Arc::new(Mutex::new(HashMap::new())),
            file_cache: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            enabled: true,
            max_entries,
            auto_cleanup,
            conflict_access_order: Arc::new(Mutex::new(VecDeque::new())),
            file_access_order: Arc::new(Mutex::new(VecDeque::new())),
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

    /// Set the maximum number of entries per cache
    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = Some(max_entries);
        self.enforce_max_entries();
    }

    /// Set auto cleanup mode
    pub fn set_auto_cleanup(&mut self, auto_cleanup: bool) {
        self.auto_cleanup = auto_cleanup;
    }
    
    /// Clear all entries from the cache
    pub fn clear(&self) {
        if let Ok(mut cache) = self.conflict_cache.lock() {
            cache.clear();
        }
        
        if let Ok(mut cache) = self.file_cache.lock() {
            cache.clear();
        }

        if let Ok(mut access_order) = self.conflict_access_order.lock() {
            access_order.clear();
        }

        if let Ok(mut access_order) = self.file_access_order.lock() {
            access_order.clear();
        }
    }

    /// Enforce maximum entries limit by removing oldest entries
    fn enforce_max_entries(&self) {
        if let Some(max) = self.max_entries {
            // Enforce for conflict cache
            if let (Ok(mut cache), Ok(mut access_order)) = (self.conflict_cache.lock(), self.conflict_access_order.lock()) {
                while access_order.len() > max {
                    if let Some(oldest) = access_order.pop_back() {
                        // Check if the entry actually exists before removing
                        if cache.contains_key(&oldest) {
                            cache.remove(&oldest);
                            debug!("Removed oldest conflict cache entry due to max size limit");
                        }
                    }
                }
            }

            // Enforce for file cache
            if let (Ok(mut cache), Ok(mut access_order)) = (self.file_cache.lock(), self.file_access_order.lock()) {
                while access_order.len() > max {
                    if let Some(oldest) = access_order.pop_back() {
                        // Check if the entry actually exists before removing
                        if cache.contains_key(&oldest) {
                            cache.remove(&oldest);
                            debug!("Removed oldest file cache entry due to max size limit");
                        }
                    }
                }
            }
        }
    }

    /// Clean up expired entries
    fn cleanup_expired(&self) {
        let now = SystemTime::now();

        // Clean up conflict cache
        if let (Ok(mut cache), Ok(mut access_order)) = (self.conflict_cache.lock(), self.conflict_access_order.lock()) {
            let mut expired_keys = Vec::new();
            
            for (key, entry) in cache.iter() {
                if entry.expires_at <= now {
                    expired_keys.push(key.clone());
                }
            }
            
            for key in &expired_keys {
                cache.remove(key);
                debug!("Removed expired conflict cache entry");
            }
            
            // Update access order
            access_order.retain(|key| !expired_keys.contains(key));
        }

        // Clean up file cache
        if let (Ok(mut cache), Ok(mut access_order)) = (self.file_cache.lock(), self.file_access_order.lock()) {
            let mut expired_keys = Vec::new();
            
            for (key, entry) in cache.iter() {
                if entry.expires_at <= now {
                    expired_keys.push(key.clone());
                }
            }
            
            for key in &expired_keys {
                cache.remove(key);
                debug!("Removed expired file cache entry");
            }
            
            // Update access order
            access_order.retain(|key| !expired_keys.contains(key));
        }
    }
    
    /// Generate a cache key for a conflict
    fn generate_conflict_key(&self, conflict: &ConflictRegion) -> String {
        // Use a hash of the combined content as the key
        // In a real implementation, we might want to use a more sophisticated
        // way to determine if conflicts are similar enough to use cached results
        format!("{}-{}-{}", 
            base64::encode(&conflict.base_content),
            base64::encode(&conflict.our_content),
            base64::encode(&conflict.their_content)
        )
    }
    
    /// Generate a cache key for a file
    fn generate_file_key(&self, file: &ConflictFile) -> String {
        // Use a hash of the file path and content as the key
        format!("{}-{}", file.path, base64::encode(&file.content))
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
        let now = SystemTime::now();
        
        if let (Ok(cache), Ok(mut access_order)) = (self.conflict_cache.lock(), self.conflict_access_order.lock()) {
            if let Some(entry) = cache.get(&key) {
                if entry.expires_at > now {
                    // Update access order - remove old position if exists
                    access_order.retain(|k| k != &key);
                    // Add to front (newest)
                    access_order.push_front(key.clone());
                    
                    debug!("Cache hit for conflict");
                    return Some(entry.response.clone());
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
        let now = SystemTime::now();
        
        if let (Ok(cache), Ok(mut access_order)) = (self.file_cache.lock(), self.file_access_order.lock()) {
            if let Some(entry) = cache.get(&key) {
                if entry.expires_at > now {
                    // Update access order - remove old position if exists
                    access_order.retain(|k| k != &key);
                    // Add to front (newest)
                    access_order.push_front(key.clone());
                    
                    debug!("Cache hit for file {}", file.path);
                    return Some(entry.response.clone());
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
        let expires_at = SystemTime::now() + self.ttl;
        
        if let (Ok(mut cache), Ok(mut access_order)) = (self.conflict_cache.lock(), self.conflict_access_order.lock()) {
            // Update access order - add/move to front of queue
            // Update access order - remove old position if exists
            access_order.retain(|k| k != &key);
            // Add to front (newest)
            access_order.push_front(key.clone());
            
            // Insert entry
            cache.insert(key, CacheEntry { response, expires_at });
            debug!("Cached response for conflict");
            
            // Enforce max entries if set
            if let Some(max) = self.max_entries {
                while access_order.len() > max {
                    if let Some(oldest) = access_order.pop_back() {
                        cache.remove(&oldest);
                        debug!("Removed oldest conflict cache entry due to max size limit");
                    }
                }
            }
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
        let expires_at = SystemTime::now() + self.ttl;
        
        if let (Ok(mut cache), Ok(mut access_order)) = (self.file_cache.lock(), self.file_access_order.lock()) {
            // Update access order - remove old position if exists
            access_order.retain(|k| k != &key);
            // Add to front (newest)
            access_order.push_front(key.clone());
            
            // Insert entry
            cache.insert(key, CacheEntry { response, expires_at });
            debug!("Cached response for file {}", file.path);
            
            // Enforce max entries if set
            if let Some(max) = self.max_entries {
                while access_order.len() > max {
                    if let Some(oldest) = access_order.pop_back() {
                        cache.remove(&oldest);
                        debug!("Removed oldest file cache entry due to max size limit");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
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
    
    #[test]
    fn test_cache_conflict_hit() {
        let cache = AIResolutionCache::new();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache
        cache.put_conflict(&conflict, response.clone());
        
        // Retrieve from cache
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_some());
        
        let cached = cached.unwrap();
        assert_eq!(cached.content, "Resolved content\n");
        assert_eq!(cached.model, "test-model");
        assert!(cached.explanation.is_some());
        assert!(cached.token_usage.is_some());
    }
    
    #[test]
    fn test_cache_conflict_miss() {
        let cache = AIResolutionCache::new();
        let conflict1 = create_test_conflict("Our content\n", "Their content\n");
        let conflict2 = create_test_conflict("Different content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache for conflict1
        cache.put_conflict(&conflict1, response);
        
        // Try to retrieve for conflict2
        let cached = cache.get_conflict(&conflict2);
        assert!(cached.is_none());
    }
    
    #[test]
    fn test_cache_file_hit() {
        let cache = AIResolutionCache::new();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict]);
        let response = create_test_response("Resolved file content\n");
        
        // Store in cache
        cache.put_file(&file, response.clone());
        
        // Retrieve from cache
        let cached = cache.get_file(&file);
        assert!(cached.is_some());
        
        let cached = cached.unwrap();
        assert_eq!(cached.content, "Resolved file content\n");
    }
    
    #[test]
    fn test_cache_expiration() {
        // Create cache with a very short TTL (1ms)
        let cache = AIResolutionCache::with_ttl(Duration::from_millis(1));
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Store in cache
        cache.put_conflict(&conflict, response);
        
        // Sleep to let it expire
        thread::sleep(Duration::from_millis(10));
        
        // Try to retrieve - should be expired
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_none());
    }
    
    #[test]
    fn test_cache_disable() {
        let mut cache = AIResolutionCache::new();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let response = create_test_response("Resolved content\n");
        
        // Disable cache
        cache.set_enabled(false);
        assert!(!cache.is_enabled());
        
        // Store in cache - should not actually store
        cache.put_conflict(&conflict, response);
        
        // Try to retrieve - should be none
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_none());
        
        // Re-enable and try again
        cache.set_enabled(true);
        assert!(cache.is_enabled());
        
        // Store in cache
        let response = create_test_response("Resolved content\n");
        cache.put_conflict(&conflict, response);
        
        // Try to retrieve - should be found
        let cached = cache.get_conflict(&conflict);
        assert!(cached.is_some());
    }
    
    #[test]
    fn test_cache_clear() {
        let cache = AIResolutionCache::new();
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict.clone()]);
        let response = create_test_response("Resolved content\n");
        
        // Store in both caches
        cache.put_conflict(&conflict, response.clone());
        cache.put_file(&file, response);
        
        // Verify both are cached
        assert!(cache.get_conflict(&conflict).is_some());
        assert!(cache.get_file(&file).is_some());
        
        // Clear cache
        cache.clear();
        
        // Verify both are cleared
        assert!(cache.get_conflict(&conflict).is_none());
        assert!(cache.get_file(&file).is_none());
    }
}