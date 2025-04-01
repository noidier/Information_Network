use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

/// Generate a random UUID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Get current time in milliseconds
pub fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Find similar paths based on string similarity
pub fn find_similar_path<T>(
    map: &HashMap<String, T>,
    target_path: &str,
    threshold: f64,
) -> Option<(String, f64)> {
    for path in map.keys() {
        let similarity = string_similarity(path, target_path);
        if similarity >= threshold {
            return Some((path.clone(), similarity));
        }
    }
    None
}

/// Calculate string similarity (Levenshtein distance)
fn string_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    
    // Special cases for empty strings
    if s1.is_empty() {
        return 0.0;
    }
    if s2.is_empty() {
        return 0.0;
    }
    
    // Simple check if one string contains the other
    if s1.contains(s2) || s2.contains(s1) {
        return 0.9;
    }
    
    // Path segment similarity - for URLs and API paths
    let segments1: Vec<&str> = s1.split('/').filter(|s| !s.is_empty()).collect();
    let segments2: Vec<&str> = s2.split('/').filter(|s| !s.is_empty()).collect();
    
    if segments1.is_empty() || segments2.is_empty() {
        return 0.0;
    }
    
    // Calculate overlap in segments
    let common_segments = segments1.iter()
        .filter(|s| segments2.contains(s))
        .count();
    
    let max_segments = segments1.len().max(segments2.len());
    
    if max_segments > 0 {
        common_segments as f64 / max_segments as f64
    } else {
        0.0
    }
}