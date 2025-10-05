use crate::cache::{self, CachedResults};
use crate::filters;
use crate::prefs::{self, Prefs};
use crate::search_runner::{RunMode, SearchOutcome};
use crate::yt::types::VideoDetails;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use egui::Context;

use super::duration_filters::{DurationFilterState, channel_sort_key};
use super::preset_editor::{PresetEditorMode, PresetEditorState};

pub enum SearchResult {
    Success(SearchOutcome),
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultSort {
    Newest,
    Oldest,
    Shortest,
    Longest,
    Channel,
}

impl ResultSort {
    pub fn label(self) -> &'static str {
        match self {
            ResultSort::Newest => "Newest",
            ResultSort::Oldest => "Oldest",
            ResultSort::Shortest => "Shortest",
            ResultSort::Longest => "Longest",
            ResultSort::Channel => "Channel",
        }
    }
}

pub struct AppState {
    pub prefs: Prefs,
    pub status: String,
    pub run_any_mode: bool,
    pub results: Vec<VideoDetails>,
    pub result_sort: ResultSort,
    pub duration_filter: DurationFilterState,
    pub runtime: Runtime,
    pub selected_search_id: Option<String>,
    pub pending_task: Option<JoinHandle<()>>,
    pub search_rx: Option<mpsc::Receiver<SearchResult>>,
    pub is_searching: bool,
    pub preset_editor: Option<PresetEditorState>,
    pub import_dialog: Option<dialogs::ImportDialogState>,
    pub export_dialog: Option<dialogs::ExportDialogState>,
    pub cached_banner_until: Option<OffsetDateTime>,
    pub show_help_dialog: bool,
}

mod dialogs;
mod preset_ops;

#[allow(unused_imports)]
pub use dialogs::{ExportDialogState, ExportMode, ImportDialogState, ImportMode};

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        super::theme::apply_gfv_theme(&cc.egui_ctx);

        let mut prefs = prefs::load_or_default();
        prefs::add_missing_defaults(&mut prefs);
        prefs::normalize_block_list(&mut prefs.blocked_channels);
        prefs::normalize_duration_filters(&mut prefs.global);
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
        let duration_filter = DurationFilterState::from_global(&prefs.global);
        let mut results: Vec<VideoDetails> = Vec::new();
        let mut cached_banner_until: Option<OffsetDateTime> = None;

        if let Some(mut cached) = cache::load_cached_results() {
            let blocked_keys = prefs::blocked_keys(&prefs.blocked_channels);
            cached.videos.retain(|video| {
                !filters::matches_channel(
                    &video.channel_handle,
                    &video.channel_title,
                    &blocked_keys,
                )
            });
            let count = cached.videos.len();
            status = if count == 0 {
                format!("Cached {} · no videos", cached.generated_at)
            } else {
                format!(
                    "Cached {} · {} video{}",
                    cached.generated_at,
                    count,
                    if count == 1 { "" } else { "s" }
                )
            };
            cached_banner_until = Some(OffsetDateTime::now_utc() + Duration::seconds(5));
            results = cached.videos;
        }

        let mut state = Self {
            prefs,
            status,
            run_any_mode: true,
            results,
            result_sort: ResultSort::Newest,
            duration_filter,
            runtime,
            selected_search_id,
            pending_task: None,
            search_rx: None,
            is_searching: false,
            preset_editor: None,
            import_dialog: None,
            export_dialog: None,
            cached_banner_until,
            show_help_dialog: false,
        };
        state.apply_result_sort();
        state
    }

    fn sync_duration_filter_to_prefs(&mut self) {
        let selected = self.duration_filter.selected_ids();
        if self.prefs.global.active_duration_bucket_ids != selected {
            self.prefs.global.active_duration_bucket_ids = selected;
        }
    }

    pub fn apply_result_sort(&mut self) {
        match self.result_sort {
            ResultSort::Newest => {
                self.results
                    .sort_by(|a, b| b.published_at.cmp(&a.published_at));
            }
            ResultSort::Oldest => {
                self.results
                    .sort_by(|a, b| a.published_at.cmp(&b.published_at));
            }
            ResultSort::Channel => {
                self.results.sort_by(|a, b| {
                    let a_key = channel_sort_key(a);
                    let b_key = channel_sort_key(b);
                    a_key
                        .cmp(&b_key)
                        .then_with(|| b.published_at.cmp(&a.published_at))
                });
            }
            ResultSort::Shortest => {
                self.results.sort_by(|a, b| {
                    a.duration_secs
                        .cmp(&b.duration_secs)
                        .then_with(|| b.published_at.cmp(&a.published_at))
                });
            }
            ResultSort::Longest => {
                self.results.sort_by(|a, b| {
                    b.duration_secs
                        .cmp(&a.duration_secs)
                        .then_with(|| b.published_at.cmp(&a.published_at))
                });
            }
        }
    }

    pub(crate) fn normalize_duration_selection(&mut self) {
        self.sync_duration_filter_to_prefs();
        prefs::normalize_duration_filters(&mut self.prefs.global);
        self.duration_filter
            .sync_from_ids(&self.prefs.global.active_duration_bucket_ids);
    }

    pub fn persist_cached_results(&self) {
        let now = OffsetDateTime::now_utc();
        let generated_at = now.format(&Rfc3339).unwrap_or_else(|_| now.to_string());
        let payload = CachedResults {
            generated_at,
            status_line: self.status.clone(),
            videos: self.results.clone(),
            saved_at_unix: now.unix_timestamp(),
        };
        if let Err(err) = cache::save_cached_results(&payload) {
            eprintln!("Failed to save cached results: {err}");
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
        self.cached_banner_until = None;

        self.normalize_duration_selection();
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
        self.apply_result_sort();
        self.cached_banner_until = None;
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

    pub fn render_help_window(&mut self, ctx: &Context) {
        if !self.show_help_dialog {
            return;
        }

        let mut open = true;
        egui::Window::new("About & Help")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .min_width(360.0)
            .show(ctx, |ui| {
                ui.heading(format!("YTSearch v{}", env!("CARGO_PKG_VERSION")));
                ui.label("A desktop helper for triaging YouTube results quickly.");

                ui.separator();
                ui.label("API key setup:");
                ui.small("1. Create a YouTube Data API v3 key in Google Cloud (enable the API)." );
                ui.small("2. Paste the key into the Settings panel (left sidebar → My Searches)." );
                ui.small("   The key is saved to prefs.json inside your YTSearch config directory.");
                ui.small("3. Press Search to fetch videos. Cached results reload automatically on startup.");

                ui.separator();
                ui.label("Documentation:");
                ui.small("• README.md → “Where to start” covers full setup details.");
                ui.small("• prefs.json lives under ~/.config/YTSearch/ (or platform equivalent).");
                ui.small("• Search results respect filters, language, and duration buckets you pick up top.");
            });

        if !open {
            self.show_help_dialog = false;
        }
    }
}
