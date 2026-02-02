use eyre::Result;
use log::{debug, info};
use std::fs;
use std::path::{Path, PathBuf};

const CACHE_DIR: &str = ".readme-cache";

/// Initializes the cache directory if it doesn't exist
pub fn init_cache() -> Result<PathBuf> {
    let cache_path = Path::new(CACHE_DIR);
    if !cache_path.exists() {
        fs::create_dir_all(cache_path)?;
        info!("created README cache directory: {}", CACHE_DIR);
    }
    Ok(cache_path.to_path_buf())
}

/// Generates a cache key from a repository name
fn get_cache_key(repo_name: &str) -> String {
    // Convert repo name to a valid filename (replace / with -)
    repo_name.replace('/', "-")
}

/// Gets the cache file path for a repository
fn get_cache_file(repo_name: &str) -> PathBuf {
    Path::new(CACHE_DIR).join(format!("{}.md", get_cache_key(repo_name)))
}

/// Retrieves a README from cache if it exists
pub fn get_cached_readme(repo_name: &str) -> Option<String> {
    let cache_file = get_cache_file(repo_name);
    if cache_file.exists() {
        match fs::read_to_string(&cache_file) {
            Ok(content) => {
                debug!("loaded README from cache: {}", repo_name);
                return Some(content);
            }
            Err(e) => {
                debug!("failed to read cached README for {}: {}", repo_name, e);
            }
        }
    }
    None
}

/// Stores a README in cache
pub fn cache_readme(repo_name: &str, content: &str) -> Result<()> {
    let cache_file = get_cache_file(repo_name);
    fs::write(&cache_file, content)?;
    debug!("cached README for: {}", repo_name);
    Ok(())
}
