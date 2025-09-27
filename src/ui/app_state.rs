use crate::filters;
use crate::prefs::{self, Prefs};
use crate::search_runner::{RunMode, SearchOutcome};
use crate::yt::types::VideoDetails;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use std::fs;
use std::path::Path;
use std::sync::mpsc;

pub enum SearchResult {
    Success(SearchOutcome),
    Error(String),
}

pub struct AppState {
    pub prefs: Prefs,
    pub status: String,
    pub run_any_mode: bool,
    pub results: Vec<VideoDetails>,
    pub runtime: Runtime,
    pub selected_search_id: Option<String>,
    pub pending_task: Option<JoinHandle<()>>,
    pub search_rx: Option<mpsc::Receiver<SearchResult>>,
    pub is_searching: bool,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        super::theme::apply_gfv_theme(&cc.egui_ctx);

        let mut prefs = prefs::load_or_default();
        prefs::add_missing_defaults(&mut prefs);
        prefs::normalize_block_list(&mut prefs.blocked_channels);
        let mut status = String::from("Ready.");

        if prefs.api_key.trim().is_empty() {
            let key_path = Path::new("YT_API_private");
            if let Ok(contents) = fs::read_to_string(key_path) {
                let trimmed = contents.trim();
                if !trimmed.is_empty() {
                    prefs.api_key = trimmed.to_owned();
                    status = "API key imported from YT_API_private.".into();
                }
            }
        }

        for search in &mut prefs.searches {
            if matches!(search.query.category_id, Some(28)) {
                search.query.category_id = None;
            }
        }
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to start tokio runtime");
        let selected_search_id = prefs.searches.first().map(|s| s.id.clone());
        Self {
            prefs,
            status,
            run_any_mode: true,
            results: Vec::new(),
            runtime,
            selected_search_id,
            pending_task: None,
            search_rx: None,
            is_searching: false,
        }
    }

    pub fn launch_search(&mut self) {
        if let Some(handle) = self.pending_task.take() {
            handle.abort();
        }
        self.search_rx = None;
        self.results.clear();
        self.status = "Searching...".into();
        self.is_searching = true;

        let prefs_snapshot = self.prefs.clone();
        let mode = match self.determine_run_mode(&prefs_snapshot) {
            Ok(mode) => mode,
            Err(msg) => {
                self.status = msg;
                self.is_searching = false;
                return;
            }
        };

        let (tx, rx) = mpsc::channel();
        let task = self.runtime.spawn(async move {
            let result = crate::search_runner::run_searches(prefs_snapshot, mode).await;
            let message = match result {
                Ok(outcome) => SearchResult::Success(outcome),
                Err(err) => SearchResult::Error(err.to_string()),
            };
            let _ = tx.send(message);
        });
        self.pending_task = Some(task);
        self.search_rx = Some(rx);
    }

    pub fn determine_run_mode(&self, prefs: &Prefs) -> Result<RunMode, String> {
        if self.run_any_mode {
            Ok(RunMode::Any)
        } else {
            let id = self
                .selected_search_id
                .clone()
                .or_else(|| prefs.searches.first().map(|s| s.id.clone()))
                .ok_or_else(|| "Add a preset before searching.".to_string())?;
            Ok(RunMode::Single(id))
        }
    }

    pub fn selected_search_name(&self) -> Option<String> {
        let target = self.selected_search_id.as_ref()?;
        self.prefs
            .searches
            .iter()
            .find(|s| &s.id == target)
            .map(|s| s.name.clone())
    }

    pub fn block_channel(&mut self, channel_id: &str, channel_title: &str) {
        let source = if !channel_id.trim().is_empty() {
            channel_id.trim()
        } else {
            channel_title.trim()
        };

        if source.is_empty() {
            self.status = "Channel identifier unavailable for blocking.".into();
            return;
        }

        let key = source.trim_start_matches('@').to_ascii_lowercase();
        if self
            .prefs
            .blocked_channels
            .iter()
            .any(|entry| prefs::parse_block_entry(entry).0 == key)
        {
            self.status = format!("Channel '{}' already blocked.", channel_title);
            return;
        }

        let label = if channel_title.trim().is_empty() {
            source.to_string()
        } else {
            channel_title.trim().to_string()
        };

        self.prefs
            .blocked_channels
            .push(format!("{}|{}", key, label));
        prefs::normalize_block_list(&mut self.prefs.blocked_channels);

        if let Err(err) = prefs::save(&self.prefs) {
            self.status = format!("Failed to save block list: {err}");
        } else {
            self.status = format!("Blocked channel: {}", channel_title);
        }

        let blocked_keys = prefs::blocked_keys(&self.prefs.blocked_channels);
        self.results.retain(|v| {
            !filters::matches_channel(&v.channel_handle, &v.channel_title, &blocked_keys)
        });
    }

    pub fn is_channel_blocked(&self, video: &VideoDetails) -> bool {
        let blocked_keys = prefs::blocked_keys(&self.prefs.blocked_channels);
        filters::matches_channel(&video.channel_handle, &video.channel_title, &blocked_keys)
    }

    pub fn unblock_channel(&mut self, channel_key: &str) {
        let target = channel_key
            .trim()
            .trim_start_matches('@')
            .to_ascii_lowercase();
        let original_len = self.prefs.blocked_channels.len();
        self.prefs
            .blocked_channels
            .retain(|entry| prefs::parse_block_entry(entry).0 != target);
        if self.prefs.blocked_channels.len() != original_len {
            prefs::normalize_block_list(&mut self.prefs.blocked_channels);
            if let Err(err) = prefs::save(&self.prefs) {
                self.status = format!("Failed to save block list: {err}");
            } else {
                self.status = format!("Unblocked channel: {}", channel_key);
            }
        }
    }
}
