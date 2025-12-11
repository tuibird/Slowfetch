// Persistent cache for slow-to-fetch OS/GPU values.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

// Global flag to force cache refresh
static FORCE_REFRESH: AtomicBool = AtomicBool::new(false);

pub fn set_force_refresh(value: bool) {
    FORCE_REFRESH.store(value, Ordering::Relaxed);
}

pub fn should_refresh() -> bool {
    FORCE_REFRESH.load(Ordering::Relaxed)
}

fn get_cache_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let cache_dir = PathBuf::from(home).join(".cache").join("slowfetch");

    // Create cache directory if it doesn't exist
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).ok()?;
    }

    Some(cache_dir)
}

fn get_cache_path(key: &str) -> Option<PathBuf> {
    Some(get_cache_dir()?.join(key))
}

// Read a cached value. Returns None if cache doesn't exist or refresh is being forced.
pub fn read_cache(key: &str) -> Option<String> {
    if should_refresh() {
        return None;
    }

    let path = get_cache_path(key)?;
    fs::read_to_string(path).ok()
}

// Write a value to cache. 10,000IQ
pub fn write_cache(key: &str, value: &str) -> Option<()> {
    let path = get_cache_path(key)?;
    fs::write(path, value).ok()
}

// Read cached GPU value, or return None to trigger the freshest of fetches.
pub fn get_cached_gpu() -> Option<String> {
    read_cache("gpu")
}

// Cache the GPU value
pub fn cache_gpu(value: &str) {
    let _ = write_cache("gpu", value);
}

// Read cached OS value, or return None to trigger a fresh fetch.
pub fn get_cached_os() -> Option<String> {
    read_cache("os")
}

// Cache the OS value (arch btw)
pub fn cache_os(value: &str) {
    let _ = write_cache("os", value);
}

// Read cached CPU value, or return None to trigger a fresh fetch.
pub fn get_cached_cpu() -> Option<String> {
    read_cache("cpu")
}

// Cache the CPU value
pub fn cache_cpu(value: &str) {
    let _ = write_cache("cpu", value);
}
