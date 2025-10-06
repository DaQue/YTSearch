use crate::cache::{self, CachedResults};
use crate::filters;
use crate::prefs::{self, Prefs};
use crate::search_runner::{RunMode, SearchOutcome};
use crate::yt::types::VideoDetails;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use egui::Context;

use super::duration_filters::{DurationFilterState, channel_sort_key};
use super::preset_editor::{PresetEditorMode, PresetEditorState};
use super::thumbnails::{self, ThumbnailRef};

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
    pub results_all: Vec<VideoDetails>,
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
    pub thumbnail_cache: thumbnails::ThumbnailCache,
}

mod dialogs;
mod preset_ops;

#[allow(unused_imports)]
pub use dialogs::{ExportDialogState, ExportMode, ImportDialogState, ImportMode};

impl AppState {
    /// Initialize UI state, loading prefs, cached results, and runtime.
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
        let duration_filter = DurationFilterState::from_global(&prefs.global);
        let mut initial_results_all: Vec<VideoDetails> = Vec::new();
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
            initial_results_all = cached.videos;
        }

        let mut state = Self {
            prefs,
            status,
            run_any_mode: true,
            results: Vec::new(),
            results_all: initial_results_all,
            result_sort: ResultSort::Newest,
            duration_filter,
            runtime,
            selected_search_id: None,
            pending_task: None,
            search_rx: None,
            is_searching: false,
            preset_editor: None,
            import_dialog: None,
            export_dialog: None,
            cached_banner_until,
            show_help_dialog: false,
            thumbnail_cache: thumbnails::ThumbnailCache::new(),
        };
        if !state.results_all.is_empty() {
            state.refresh_visible_results();
        } else {
            state.apply_result_sort();
        }
        state.sync_thumbnail_cache();
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

    /// Drop cached textures for videos that are no longer present.
    pub(super) fn sync_thumbnail_cache(&mut self) {
        let ids = self.results_all.iter().map(|video| video.id.as_str());
        self.thumbnail_cache.retain_ids(ids);
    }

    /// Request or fetch a thumbnail for the given video, returning it if ready.
    pub fn thumbnail_for_video(
        &mut self,
        ctx: &Context,
        video: &VideoDetails,
    ) -> Option<ThumbnailRef> {
        self.thumbnail_cache.request(
            &video.id,
            video.thumbnail_url.as_deref(),
            ctx,
            &self.runtime,
        );
        self.thumbnail_cache.thumbnail(&video.id)
    }

    /// Restore built-in presets while keeping API key/min duration, clearing cache/state.
    pub fn reset_to_defaults(&mut self) {
        let saved_api_key = self.prefs.api_key.clone();
        let saved_min_duration = self.prefs.global.min_duration_secs;

        let mut defaults = prefs::builtin_default();
        defaults.api_key = saved_api_key;
        defaults.blocked_channels.clear();
        defaults.global.min_duration_secs = saved_min_duration;
        defaults.global.active_duration_bucket_ids =
            defaults.global.duration_filters.default_active_ids();

        prefs::normalize_duration_filters(&mut defaults.global);
        prefs::normalize_block_list(&mut defaults.blocked_channels);

        self.prefs = defaults;
        self.duration_filter = DurationFilterState::from_global(&self.prefs.global);
        self.results.clear();
        self.results_all.clear();
        self.thumbnail_cache.clear();
        self.sync_thumbnail_cache();
        self.selected_search_id = None;
        self.apply_result_sort();
        self.cached_banner_until = None;
        self.status = "Defaults restored. Adjust filters and search.".into();

        if let Err(err) = prefs::save(&self.prefs) {
            self.status = format!("Defaults restored, but failed to save: {err}");
        }
    }

    /// Persist duration filter selections back into preferences.
    pub(crate) fn normalize_duration_selection(&mut self) {
        self.sync_duration_filter_to_prefs();
        prefs::normalize_duration_filters(&mut self.prefs.global);
        self.duration_filter
            .sync_from_ids(&self.prefs.global.active_duration_bucket_ids);
    }

    /// Recalculate visible results based on run mode and preset selection.
    pub fn refresh_visible_results(&mut self) {
        let mut filtered: Vec<VideoDetails> = Vec::new();
        if self.run_any_mode {
            let enabled_names: HashSet<&str> = self
                .prefs
                .searches
                .iter()
                .filter(|preset| preset.enabled)
                .map(|preset| preset.name.as_str())
                .collect();
            if enabled_names.is_empty() {
                self.results.clear();
                return;
            }
            for video in &self.results_all {
                if video
                    .source_presets
                    .iter()
                    .any(|name| enabled_names.contains(name.as_str()))
                {
                    filtered.push(video.clone());
                }
            }
        } else {
            if let Some(selected_id) = self.selected_search_id.clone() {
                if let Some(selected_preset) = self
                    .prefs
                    .searches
                    .iter()
                    .find(|preset| preset.id == selected_id)
                {
                    for video in &self.results_all {
                        if video
                            .source_presets
                            .iter()
                            .any(|name| name == &selected_preset.name)
                        {
                            filtered.push(video.clone());
                        }
                    }
                } else {
                    self.selected_search_id = None;
                    filtered = self.results_all.clone();
                }
            } else {
                filtered = self.results_all.clone();
            }
        }

        self.results = filtered;
        self.apply_result_sort();
    }

    /// Write current results to disk so next launch can reuse them.
    pub fn persist_cached_results(&self) {
        let now = OffsetDateTime::now_utc();
        let generated_at = now.format(&Rfc3339).unwrap_or_else(|_| now.to_string());
        let payload = CachedResults {
            generated_at,
            status_line: self.status.clone(),
            videos: self.results_all.clone(),
            saved_at_unix: now.unix_timestamp(),
        };
        if let Err(err) = cache::save_cached_results(&payload) {
            eprintln!("Failed to save cached results: {err}");
        }
    }

    /// Start an async search task using current prefs and UI state.
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

    /// Derive run mode from UI state, falling back to Any if nothing is selected.
    pub fn determine_run_mode(&self, prefs: &Prefs) -> Result<RunMode, String> {
        if self.run_any_mode {
            Ok(RunMode::Any)
        } else {
            if let Some(id) = self.selected_search_id.clone() {
                Ok(RunMode::Single(id))
            } else if prefs.searches.is_empty() {
                Ok(RunMode::Any)
            } else {
                Ok(RunMode::Any)
            }
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
