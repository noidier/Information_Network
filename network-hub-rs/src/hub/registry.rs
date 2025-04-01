use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::utils::find_similar_path;
use crate::hub::types::ApiRequest;
use crate::hub::types::ApiResponse;

/// A registered API handler
pub struct ApiEntry {
    /// The handler function
    pub handler: Arc<dyn Fn(&ApiRequest) -> ApiResponse + Send + Sync>,
    /// Metadata about the API
    pub metadata: HashMap<String, String>,
    /// Optional fallback path if this API is not available
    pub fallback_path: Option<String>,
}

/// Registry of API endpoints
pub struct ApiRegistry {
    /// Map of API paths to handlers
    entries: RwLock<HashMap<String, ApiEntry>>,
}

impl ApiRegistry {
    /// Create a new API registry
    pub fn new() -> Self {
        ApiRegistry {
            entries: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register an API handler
    pub fn register<F>(&self, path: &str, handler: F, metadata: HashMap<String, String>)
    where
        F: Fn(&ApiRequest) -> ApiResponse + Send + Sync + 'static,
    {
        let fallback_path = metadata.get("fallback").cloned();
        
        let entry = ApiEntry {
            handler: Arc::new(handler),
            metadata,
            fallback_path,
        };
        
        let mut entries = self.entries.write().unwrap();
        entries.insert(path.to_string(), entry);
    }
    
    /// Look up an API handler by path
    pub fn lookup(&self, path: &str) -> Option<ApiEntry> {
        let entries = self.entries.read().unwrap();
        entries.get(path).cloned()
    }
    
    /// Look up a fallback path for an API
    pub fn lookup_fallback(&self, path: &str) -> Option<(String, ApiEntry)> {
        let entries = self.entries.read().unwrap();
        
        for (api_path, entry) in entries.iter() {
            if let Some(fallback) = &entry.fallback_path {
                if fallback == path {
                    return Some((api_path.clone(), entry.clone()));
                }
            }
        }
        
        None
    }
    
    /// Look up an API with a similar path
    pub fn lookup_similar(&self, path: &str, threshold: f64) -> Option<(String, ApiEntry)> {
        let entries = self.entries.read().unwrap();
        let entries_map = entries.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<_, _>>();
        
        if let Some((similar_path, _)) = find_similar_path(&entries_map, path, threshold) {
            return entries.get(&similar_path).map(|entry| (similar_path, entry.clone()));
        }
        
        None
    }
}

impl Clone for ApiEntry {
    fn clone(&self) -> Self {
        ApiEntry {
            handler: Arc::clone(&self.handler),
            metadata: self.metadata.clone(),
            fallback_path: self.fallback_path.clone(),
        }
    }
}