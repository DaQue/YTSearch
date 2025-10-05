use crate::yt::types::VideoDetails;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResults {
    pub generated_at: String,
    pub status_line: String,
    pub videos: Vec<VideoDetails>,
    #[serde(default = "default_saved_at")]
    pub saved_at_unix: i64,
}

fn default_saved_at() -> i64 {
    0
}

fn cache_path() -> PathBuf {
    let proj = ProjectDirs::from("com", "yourname", "YTSearch").expect("no project dirs");
    proj.config_dir().join("last_results.json")
}

pub fn load_cached_results() -> Option<CachedResults> {
    let path = cache_path();
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<CachedResults>(&bytes).ok()
}

pub fn save_cached_results(results: &CachedResults) -> std::io::Result<()> {
    let path = cache_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_vec_pretty(results)?)
}

pub fn clear_cached_results() -> std::io::Result<()> {
    let path = cache_path();
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
