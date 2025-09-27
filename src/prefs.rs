use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, mem, path::PathBuf};

const DEFAULT_PREFS_JSON: &str = include_str!("prefs_defaults.json");

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Prefs {
    pub api_key: String,
    pub global: GlobalPrefs,
    pub searches: Vec<MySearch>,
    pub blocked_channels: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct GlobalPrefs {
    pub default_window: TimeWindowPreset,
    pub english_only: bool,
    pub require_captions: bool,
    pub verify_captions_with_oauth: bool,
    pub min_duration_secs: u32,
    pub region_code: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct MySearch {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub query: QuerySpec,
    pub window_override: Option<TimeWindow>,
    pub english_only_override: Option<bool>,
    pub require_captions_override: Option<bool>,
    pub min_duration_override: Option<u32>,
    pub priority: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct QuerySpec {
    pub q: Option<String>,
    pub any_terms: Vec<String>,
    pub all_terms: Vec<String>,
    pub not_terms: Vec<String>,
    pub channel_allow: Vec<String>,
    pub channel_deny: Vec<String>,
    pub category_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TimeWindowPreset {
    Today,
    H48,
    #[default]
    D7,
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeWindow {
    pub start_rfc3339: String,
    pub end_rfc3339: String,
}

impl Default for GlobalPrefs {
    fn default() -> Self {
        Self {
            default_window: TimeWindowPreset::default(),
            english_only: true,
            require_captions: false,
            verify_captions_with_oauth: false,
            min_duration_secs: 75,
            region_code: Some("US".into()),
        }
    }
}

pub fn load_or_default() -> Prefs {
    let path = prefs_path();
    let mut prefs = if let Ok(bytes) = fs::read(&path) {
        serde_json::from_slice::<Prefs>(&bytes).unwrap_or_else(|_| builtin_default())
    } else {
        builtin_default()
    };
    add_missing_defaults(&mut prefs);
    normalize_block_list(&mut prefs.blocked_channels);
    prefs
}

pub fn save(p: &Prefs) -> std::io::Result<()> {
    let path = prefs_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_vec_pretty(p)?)
}

fn prefs_path() -> PathBuf {
    let proj = ProjectDirs::from("com", "yourname", "YTSearch").expect("no project dirs");
    proj.config_dir().join("prefs.json")
}

fn builtin_default() -> Prefs {
    serde_json::from_str(DEFAULT_PREFS_JSON).unwrap_or_default()
}

pub fn add_missing_defaults(prefs: &mut Prefs) {
    let defaults = builtin_default();
    for default_search in defaults.searches {
        if !prefs.searches.iter().any(|s| s.id == default_search.id) {
            prefs.searches.push(default_search);
        }
    }
}

pub fn normalize_block_list(list: &mut Vec<String>) {
    let mut map = BTreeMap::new();
    for entry in mem::take(list) {
        let (key, label) = parse_block_entry(&entry);
        if key.is_empty() {
            continue;
        }
        map.entry(key.clone())
            .or_insert_with(|| format!("{}|{}", key, label));
    }
    *list = map.into_values().collect();
}

pub fn blocked_keys(entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| parse_block_entry(entry).0)
        .collect()
}

pub fn parse_block_entry(entry: &str) -> (String, String) {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }
    if let Some((raw_key, raw_label)) = trimmed.split_once('|') {
        let key = raw_key.trim().trim_start_matches('@').to_ascii_lowercase();
        let label = raw_label.trim();
        let label = if label.is_empty() {
            raw_key.trim().to_string()
        } else {
            label.to_string()
        };
        (key, label)
    } else {
        let key = trimmed.trim_start_matches('@').to_ascii_lowercase();
        (key.clone(), trimmed.to_string())
    }
}
