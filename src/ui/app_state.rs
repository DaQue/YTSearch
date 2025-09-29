use crate::filters;
use crate::prefs::{self, MySearch, Prefs};
use crate::search_runner::{RunMode, SearchOutcome};
use crate::yt::types::VideoDetails;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use time::OffsetDateTime;

pub enum SearchResult {
    Success(SearchOutcome),
    Error(String),
}

#[derive(Clone)]
pub enum PresetEditorMode {
    New,
    Edit { index: usize },
    Duplicate { source_index: usize },
}

pub struct PresetEditorState {
    pub mode: PresetEditorMode,
    pub working: MySearch,
    pub enabled: bool,
    pub name: String,
    pub query_text: String,
    pub any_terms: Vec<String>,
    pub new_any_term: String,
    pub all_terms: Vec<String>,
    pub new_all_term: String,
    pub not_terms: Vec<String>,
    pub new_not_term: String,
    pub channel_allow: Vec<String>,
    pub new_allow_entry: String,
    pub channel_deny: Vec<String>,
    pub new_deny_entry: String,
    pub window_override_enabled: bool,
    pub window_start: String,
    pub window_end: String,
    pub english_override_enabled: bool,
    pub english_override_value: bool,
    pub captions_override_enabled: bool,
    pub captions_override_value: bool,
    pub min_duration_override_enabled: bool,
    pub min_duration_override_value: u32,
    pub priority: i32,
    pub error: Option<String>,
}

impl PresetEditorState {
    pub fn new(
        mode: PresetEditorMode,
        source: &MySearch,
        default_english: bool,
        default_captions: bool,
        default_min_duration: u32,
    ) -> Self {
        let mut working = source.clone();
        if !matches!(mode, PresetEditorMode::Edit { .. }) {
            working.id = String::new();
            working.enabled = true;
        }

        let (window_enabled, start, end) = if let Some(window) = &working.window_override {
            (
                true,
                window.start_rfc3339.clone(),
                window.end_rfc3339.clone(),
            )
        } else {
            (false, String::new(), String::new())
        };

        let english_enabled = working.english_only_override.is_some();
        let english_value = working.english_only_override.unwrap_or(default_english);
        let captions_enabled = working.require_captions_override.is_some();
        let captions_value = working
            .require_captions_override
            .unwrap_or(default_captions);
        let min_duration_enabled = working.min_duration_override.is_some();
        let min_duration_value = working
            .min_duration_override
            .unwrap_or(default_min_duration);

        let mut any_terms = working.query.any_terms.clone();
        let mut all_terms = working.query.all_terms.clone();
        let mut not_terms = working.query.not_terms.clone();
        let mut channel_allow = working.query.channel_allow.clone();
        let mut channel_deny = working.query.channel_deny.clone();

        Self::normalize_terms(&mut any_terms);
        Self::normalize_terms(&mut all_terms);
        Self::normalize_terms(&mut not_terms);
        Self::normalize_terms(&mut channel_allow);
        Self::normalize_terms(&mut channel_deny);

        Self {
            mode,
            enabled: working.enabled,
            name: working.name.clone(),
            query_text: working.query.q.clone().unwrap_or_default(),
            any_terms,
            new_any_term: String::new(),
            all_terms,
            new_all_term: String::new(),
            not_terms,
            new_not_term: String::new(),
            channel_allow,
            new_allow_entry: String::new(),
            channel_deny,
            new_deny_entry: String::new(),
            window_override_enabled: window_enabled,
            window_start: start,
            window_end: end,
            english_override_enabled: english_enabled,
            english_override_value: english_value,
            captions_override_enabled: captions_enabled,
            captions_override_value: captions_value,
            min_duration_override_enabled: min_duration_enabled,
            min_duration_override_value: min_duration_value,
            priority: working.priority,
            working,
            error: None,
        }
    }

    pub(crate) fn normalize_terms(list: &mut Vec<String>) {
        let mut seen = HashSet::new();
        list.retain(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return false;
            }
            let normalized = trimmed.to_string();
            if seen.contains(&normalized) {
                return false;
            }
            seen.insert(normalized.clone());
            true
        });
        for item in list.iter_mut() {
            *item = item.trim().to_string();
        }
    }

    pub fn hydrate_working(&mut self) {
        self.working.name = self.name.trim().to_string();
        self.working.enabled = self.enabled;
        let query = &mut self.working.query;
        query.q = if self.query_text.trim().is_empty() {
            None
        } else {
            Some(self.query_text.trim().to_string())
        };
        Self::normalize_terms(&mut self.any_terms);
        Self::normalize_terms(&mut self.all_terms);
        Self::normalize_terms(&mut self.not_terms);
        Self::normalize_terms(&mut self.channel_allow);
        Self::normalize_terms(&mut self.channel_deny);
        query.any_terms = self.any_terms.clone();
        query.all_terms = self.all_terms.clone();
        query.not_terms = self.not_terms.clone();
        query.channel_allow = self.channel_allow.clone();
        query.channel_deny = self.channel_deny.clone();

        if self.window_override_enabled {
            if self.window_start.trim().is_empty() || self.window_end.trim().is_empty() {
                self.working.window_override = None;
            } else {
                self.working.window_override = Some(crate::prefs::TimeWindow {
                    start_rfc3339: self.window_start.trim().to_string(),
                    end_rfc3339: self.window_end.trim().to_string(),
                });
            }
        } else {
            self.working.window_override = None;
        }

        self.working.english_only_override = if self.english_override_enabled {
            Some(self.english_override_value)
        } else {
            None
        };

        self.working.require_captions_override = if self.captions_override_enabled {
            Some(self.captions_override_value)
        } else {
            None
        };

        self.working.min_duration_override = if self.min_duration_override_enabled {
            Some(self.min_duration_override_value)
        } else {
            None
        };

        self.working.priority = self.priority;
    }
}

pub struct ImportDialogState {
    pub raw_json: String,
    pub error: Option<String>,
}

pub struct ExportDialogState {
    pub raw_json: String,
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
    pub preset_editor: Option<PresetEditorState>,
    pub import_dialog: Option<ImportDialogState>,
    pub export_dialog: Option<ExportDialogState>,
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
            preset_editor: None,
            import_dialog: None,
            export_dialog: None,
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

    pub fn open_new_preset(&mut self) {
        let mut template = MySearch::default();
        template.priority = self.prefs.searches.len() as i32;
        let editor = PresetEditorState::new(
            PresetEditorMode::New,
            &template,
            self.prefs.global.english_only,
            self.prefs.global.require_captions,
            self.prefs.global.min_duration_secs,
        );
        self.preset_editor = Some(editor);
    }

    pub fn open_edit_preset(&mut self, index: usize) {
        if let Some(existing) = self.prefs.searches.get(index) {
            let editor = PresetEditorState::new(
                PresetEditorMode::Edit { index },
                existing,
                self.prefs.global.english_only,
                self.prefs.global.require_captions,
                self.prefs.global.min_duration_secs,
            );
            self.preset_editor = Some(editor);
        }
    }

    pub fn open_duplicate_preset(&mut self, index: usize) {
        if let Some(existing) = self.prefs.searches.get(index) {
            let mut duplicate = existing.clone();
            if !duplicate.name.trim().is_empty() {
                duplicate.name = format!("{} copy", duplicate.name.trim());
            }
            let mut editor = PresetEditorState::new(
                PresetEditorMode::Duplicate {
                    source_index: index,
                },
                &duplicate,
                self.prefs.global.english_only,
                self.prefs.global.require_captions,
                self.prefs.global.min_duration_secs,
            );
            if editor.name.trim().is_empty() {
                editor.name = "New preset".into();
            }
            self.preset_editor = Some(editor);
        }
    }

    pub fn delete_preset(&mut self, index: usize) {
        if index >= self.prefs.searches.len() {
            return;
        }
        let removed = self.prefs.searches.remove(index);
        if let Err(err) = prefs::save(&self.prefs) {
            self.status = format!("Failed to save prefs: {err}");
        } else {
            self.status = format!("Removed preset '{}'.", removed.name);
        }

        if let Some(selected) = self.selected_search_id.clone() {
            if selected == removed.id {
                let next_id = if index < self.prefs.searches.len() {
                    Some(self.prefs.searches[index].id.clone())
                } else {
                    self.prefs.searches.last().map(|s| s.id.clone())
                };
                self.selected_search_id = next_id;
            }
        }
    }

    pub fn cancel_editor(&mut self) {
        self.preset_editor = None;
    }

    pub fn try_save_editor(&mut self) {
        let Some(mut editor) = self.preset_editor.take() else {
            return;
        };

        editor.error = None;
        editor.hydrate_working();

        if editor.name.trim().is_empty() {
            editor.error = Some("Name cannot be empty.".into());
            self.preset_editor = Some(editor);
            return;
        }

        let mut working = editor.working.clone();
        let has_query_text = working
            .query
            .q
            .as_ref()
            .map(|q| !q.trim().is_empty())
            .unwrap_or(false);
        if !has_query_text
            && working.query.any_terms.is_empty()
            && working.query.all_terms.is_empty()
        {
            editor.error = Some("Configure at least one query term.".into());
            self.preset_editor = Some(editor);
            return;
        }

        enum SaveAction {
            Update {
                index: usize,
                id: String,
                preset: MySearch,
            },
            Append {
                preset: MySearch,
            },
        }

        let action = match editor.mode {
            PresetEditorMode::Edit { index } => {
                if index >= self.prefs.searches.len() {
                    editor.error = Some("Preset no longer exists.".into());
                    self.preset_editor = Some(editor);
                    return;
                }
                let id = self.prefs.searches[index].id.clone();
                working.id = id.clone();
                SaveAction::Update {
                    index,
                    id,
                    preset: working,
                }
            }
            _ => {
                let generated = self.generate_unique_id(&working.name);
                working.id = generated.clone();
                SaveAction::Append { preset: working }
            }
        };

        match &action {
            SaveAction::Update { index, .. } => {
                if *index >= self.prefs.searches.len() {
                    editor.error = Some("Preset no longer exists.".into());
                    self.preset_editor = Some(editor);
                    return;
                }
            }
            SaveAction::Append { .. } => {}
        }

        match action {
            SaveAction::Update { index, id, preset } => {
                self.prefs.searches[index] = preset;
                self.selected_search_id = Some(id);
                self.status = "Preset updated.".into();
            }
            SaveAction::Append { preset } => {
                self.selected_search_id = Some(preset.id.clone());
                self.prefs.searches.push(preset);
                self.status = "Preset added.".into();
            }
        }

        if let Err(err) = prefs::save(&self.prefs) {
            editor.error = Some(format!("Failed to save prefs: {err}"));
            self.preset_editor = Some(editor);
            return;
        }

        self.preset_editor = None;
    }

    fn generate_unique_id(&self, name: &str) -> String {
        let mut base: String = name
            .trim()
            .to_ascii_lowercase()
            .chars()
            .map(|ch| match ch {
                'a'..='z' | '0'..='9' => ch,
                _ => '-',
            })
            .collect();
        while base.contains("--") {
            base = base.replace("--", "-");
        }
        base = base.trim_matches('-').to_string();
        if base.is_empty() {
            base = format!("preset-{}", OffsetDateTime::now_utc().unix_timestamp());
        }
        let mut candidate = base.clone();
        let mut counter = 2usize;
        while self.prefs.searches.iter().any(|s| s.id == candidate) {
            candidate = format!("{}-{}", base, counter);
            counter += 1;
        }
        candidate
    }

    pub fn open_import_dialog(&mut self) {
        self.import_dialog = Some(ImportDialogState {
            raw_json: String::new(),
            error: None,
        });
    }

    pub fn open_export_dialog(&mut self) {
        match serde_json::to_string_pretty(&self.prefs.searches) {
            Ok(raw_json) => {
                self.export_dialog = Some(ExportDialogState { raw_json });
            }
            Err(err) => {
                self.status = format!("Export failed: {err}");
            }
        }
    }

    pub fn cancel_import_dialog(&mut self) {
        self.import_dialog = None;
    }

    pub fn cancel_export_dialog(&mut self) {
        self.export_dialog = None;
    }

    pub fn apply_import(&mut self) {
        let Some(mut dialog) = self.import_dialog.take() else {
            return;
        };

        dialog.error = None;
        let parsed: Result<Vec<MySearch>, _> = serde_json::from_str(&dialog.raw_json);
        let mut presets = match parsed {
            Ok(list) => list,
            Err(_) => match serde_json::from_str::<Prefs>(&dialog.raw_json) {
                Ok(prefs_payload) => prefs_payload.searches,
                Err(err) => {
                    dialog.error = Some(format!("Import failed: {err}"));
                    self.import_dialog = Some(dialog);
                    return;
                }
            },
        };

        if presets.is_empty() {
            dialog.error = Some("No presets found in import.".into());
            self.import_dialog = Some(dialog);
            return;
        }

        for preset in &mut presets {
            preset.name = preset.name.trim().to_string();
        }

        let mut added = 0usize;
        for mut preset in presets {
            if preset.name.is_empty() {
                continue;
            }
            if preset.id.trim().is_empty() || self.prefs.searches.iter().any(|s| s.id == preset.id)
            {
                preset.id = self.generate_unique_id(&preset.name);
            }
            self.prefs.searches.push(preset);
            added += 1;
        }

        if added == 0 {
            dialog.error = Some("No valid presets to import.".into());
            self.import_dialog = Some(dialog);
            return;
        }

        if let Err(err) = prefs::save(&self.prefs) {
            dialog.error = Some(format!("Failed to save prefs: {err}"));
            self.import_dialog = Some(dialog);
            return;
        }

        self.status = format!("Imported {added} preset(s).");
        self.selected_search_id = self.prefs.searches.last().map(|s| s.id.clone());
    }
}
