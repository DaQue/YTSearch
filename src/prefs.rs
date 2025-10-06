use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, mem, path::PathBuf};

const DEFAULT_PREFS_JSON: &str = include_str!("prefs_defaults.json");

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Prefs {
    pub api_key: String,
    pub global: GlobalPrefs,
    pub searches: Vec<MySearch>,
    pub blocked_channels: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct GlobalPrefs {
    pub default_window: TimeWindowPreset,
    pub english_only: bool,
    pub require_captions: bool,
    pub verify_captions_with_oauth: bool,
    pub min_duration_secs: u32,
    pub duration_filters: DurationFilterConfig,
    pub active_duration_bucket_ids: Vec<String>,
    pub region_code: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct DurationFilterConfig {
    pub buckets: Vec<DurationBucketConfig>,
    pub allow_multiple: bool,
}

impl Default for DurationFilterConfig {
    fn default() -> Self {
        Self {
            allow_multiple: true,
            buckets: vec![
                DurationBucketConfig {
                    id: "any".into(),
                    label: "Any length".into(),
                    min_seconds: 0,
                    max_seconds: None,
                    default_selected: true,
                },
                DurationBucketConfig {
                    id: "shorts".into(),
                    label: "Shorts (<3 min)".into(),
                    min_seconds: 0,
                    max_seconds: Some(180),
                    default_selected: false,
                },
                DurationBucketConfig {
                    id: "brief".into(),
                    label: "Brief (3-15 min)".into(),
                    min_seconds: 180,
                    max_seconds: Some(900),
                    default_selected: false,
                },
                DurationBucketConfig {
                    id: "medium".into(),
                    label: "Medium (15-30 min)".into(),
                    min_seconds: 900,
                    max_seconds: Some(1800),
                    default_selected: false,
                },
                DurationBucketConfig {
                    id: "long".into(),
                    label: "Long (30-60 min)".into(),
                    min_seconds: 1800,
                    max_seconds: Some(3600),
                    default_selected: false,
                },
                DurationBucketConfig {
                    id: "very-long".into(),
                    label: "Very Long (60+ min)".into(),
                    min_seconds: 3600,
                    max_seconds: None,
                    default_selected: false,
                },
            ],
        }
    }
}

impl DurationFilterConfig {
    pub fn bucket_by_id(&self, id: &str) -> Option<&DurationBucketConfig> {
        self.buckets.iter().find(|bucket| bucket.id == id)
    }

    pub fn default_active_ids(&self) -> Vec<String> {
        let mut defaults: Vec<String> = self
            .buckets
            .iter()
            .filter(|bucket| bucket.default_selected)
            .map(|bucket| bucket.id.clone())
            .collect();
        if defaults.is_empty() {
            if let Some(catch_all) = self.buckets.iter().find(|bucket| bucket.is_catch_all()) {
                defaults.push(catch_all.id.clone());
            } else if let Some(first) = self.buckets.first() {
                defaults.push(first.id.clone());
            }
        }
        defaults
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct DurationBucketConfig {
    pub id: String,
    pub label: String,
    pub min_seconds: u32,
    pub max_seconds: Option<u32>,
    pub default_selected: bool,
}

impl Default for DurationBucketConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            min_seconds: 0,
            max_seconds: None,
            default_selected: true,
        }
    }
}

impl DurationBucketConfig {
    pub fn contains(&self, secs: u64) -> bool {
        if secs < self.min_seconds as u64 {
            return false;
        }
        if let Some(max) = self.max_seconds {
            if secs >= max as u64 {
                return false;
            }
        }
        true
    }

    pub fn is_catch_all(&self) -> bool {
        self.min_seconds == 0 && self.max_seconds.is_none()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
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
    pub system: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
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
    #[serde(alias = "Custom")]
    AllTime,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TimeWindow {
    pub start_rfc3339: String,
    pub end_rfc3339: String,
}

impl Default for GlobalPrefs {
    fn default() -> Self {
        let duration_filters = DurationFilterConfig::default();
        let active_duration_bucket_ids = duration_filters.default_active_ids();
        Self {
            default_window: TimeWindowPreset::default(),
            english_only: true,
            require_captions: false,
            verify_captions_with_oauth: false,
            min_duration_secs: 75,
            duration_filters,
            active_duration_bucket_ids,
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
    normalize_duration_filters(&mut prefs.global);
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

pub fn builtin_default() -> Prefs {
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

pub fn normalize_duration_filters(global: &mut GlobalPrefs) {
    let config = &global.duration_filters;
    let mut active: Vec<String> = Vec::new();
    for bucket in &config.buckets {
        if global
            .active_duration_bucket_ids
            .iter()
            .any(|id| id == &bucket.id)
        {
            active.push(bucket.id.clone());
            if !config.allow_multiple {
                break;
            }
        }
    }
    if active.is_empty() {
        active = config.default_active_ids();
    }
    if !config.allow_multiple && active.len() > 1 {
        active.truncate(1);
    }
    global.active_duration_bucket_ids = active;
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
