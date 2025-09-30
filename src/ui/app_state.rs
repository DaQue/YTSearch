use crate::filters;
use crate::prefs::{self, MySearch, Prefs};
use crate::search_runner::{RunMode, SearchOutcome};
use crate::yt::types::VideoDetails;
use anyhow::bail;
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

#[derive(Clone)]
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
    pub default_english: bool,
    pub default_captions: bool,
    pub default_min_duration: u32,
    pub initial: MySearch,
    pub awaiting_clipboard: bool,
    pub pending_clipboard: Option<MySearch>,
    pub show_dirty_warning: bool,
}

impl PresetEditorState {
    pub fn new(
        mode: PresetEditorMode,
        source: &MySearch,
        default_english: bool,
        default_captions: bool,
        default_min_duration: u32,
    ) -> Self {
        let mut state = Self {
            mode,
            working: MySearch::default(),
            enabled: true,
            name: String::new(),
            query_text: String::new(),
            any_terms: Vec::new(),
            new_any_term: String::new(),
            all_terms: Vec::new(),
            new_all_term: String::new(),
            not_terms: Vec::new(),
            new_not_term: String::new(),
            channel_allow: Vec::new(),
            new_allow_entry: String::new(),
            channel_deny: Vec::new(),
            new_deny_entry: String::new(),
            window_override_enabled: false,
            window_start: String::new(),
            window_end: String::new(),
            english_override_enabled: false,
            english_override_value: default_english,
            captions_override_enabled: false,
            captions_override_value: default_captions,
            min_duration_override_enabled: false,
            min_duration_override_value: default_min_duration,
            priority: 0,
            error: None,
            default_english,
            default_captions,
            default_min_duration,
            initial: MySearch::default(),
            awaiting_clipboard: false,
            pending_clipboard: None,
            show_dirty_warning: false,
        };
        state.apply_source(source);
        state.initial = state.snapshot();
        state.working = state.initial.clone();
        state
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

    fn normalized_terms_vec(tokens: &[String]) -> Vec<String> {
        let mut copy = tokens.to_vec();
        Self::normalize_terms(&mut copy);
        copy
    }

    fn apply_terms_to_self(
        &mut self,
    ) -> (
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
    ) {
        let any_terms = Self::normalized_terms_vec(&self.any_terms);
        let all_terms = Self::normalized_terms_vec(&self.all_terms);
        let not_terms = Self::normalized_terms_vec(&self.not_terms);
        let channel_allow = Self::normalized_terms_vec(&self.channel_allow);
        let channel_deny = Self::normalized_terms_vec(&self.channel_deny);

        self.any_terms = any_terms.clone();
        self.all_terms = all_terms.clone();
        self.not_terms = not_terms.clone();
        self.channel_allow = channel_allow.clone();
        self.channel_deny = channel_deny.clone();

        (any_terms, all_terms, not_terms, channel_allow, channel_deny)
    }

    fn populate_target(
        &self,
        target: &mut MySearch,
        any_terms: &[String],
        all_terms: &[String],
        not_terms: &[String],
        channel_allow: &[String],
        channel_deny: &[String],
    ) {
        target.name = self.name.trim().to_string();
        target.enabled = self.enabled;
        let trimmed_query = self.query_text.trim();
        target.query.q = if trimmed_query.is_empty() {
            None
        } else {
            Some(trimmed_query.to_string())
        };
        target.query.any_terms = any_terms.to_vec();
        target.query.all_terms = all_terms.to_vec();
        target.query.not_terms = not_terms.to_vec();
        target.query.channel_allow = channel_allow.to_vec();
        target.query.channel_deny = channel_deny.to_vec();

        if self.window_override_enabled
            && !self.window_start.trim().is_empty()
            && !self.window_end.trim().is_empty()
        {
            target.window_override = Some(crate::prefs::TimeWindow {
                start_rfc3339: self.window_start.trim().to_string(),
                end_rfc3339: self.window_end.trim().to_string(),
            });
        } else {
            target.window_override = None;
        }

        target.english_only_override = if self.english_override_enabled {
            Some(self.english_override_value)
        } else {
            None
        };

        target.require_captions_override = if self.captions_override_enabled {
            Some(self.captions_override_value)
        } else {
            None
        };

        target.min_duration_override = if self.min_duration_override_enabled {
            Some(self.min_duration_override_value)
        } else {
            None
        };

        target.priority = self.priority;
    }

    pub fn hydrate_working(&mut self) {
        let (any_terms, all_terms, not_terms, channel_allow, channel_deny) =
            self.apply_terms_to_self();
        let mut target = self.working.clone();
        self.populate_target(
            &mut target,
            &any_terms,
            &all_terms,
            &not_terms,
            &channel_allow,
            &channel_deny,
        );
        self.working = target;
    }

    pub fn snapshot(&self) -> MySearch {
        let mut cloned = self.clone();
        cloned.hydrate_working();
        cloned.working
    }

    pub fn is_dirty(&self) -> bool {
        self.snapshot() != self.initial
    }

    pub fn reset_dirty_baseline(&mut self) {
        self.hydrate_working();
        self.initial = self.working.clone();
    }

    pub fn apply_source(&mut self, source: &MySearch) {
        self.working = source.clone();
        if !matches!(self.mode, PresetEditorMode::Edit { .. }) {
            self.working.id = self.working.id.trim().to_string();
            self.working.enabled = true;
        }

        let working = &self.working;
        self.enabled = working.enabled;
        self.name = working.name.clone();
        self.query_text = working.query.q.clone().unwrap_or_default();

        self.any_terms = working.query.any_terms.clone();
        self.new_any_term.clear();
        self.all_terms = working.query.all_terms.clone();
        self.new_all_term.clear();
        self.not_terms = working.query.not_terms.clone();
        self.new_not_term.clear();
        self.channel_allow = working.query.channel_allow.clone();
        self.new_allow_entry.clear();
        self.channel_deny = working.query.channel_deny.clone();
        self.new_deny_entry.clear();

        Self::normalize_terms(&mut self.any_terms);
        Self::normalize_terms(&mut self.all_terms);
        Self::normalize_terms(&mut self.not_terms);
        Self::normalize_terms(&mut self.channel_allow);
        Self::normalize_terms(&mut self.channel_deny);

        if let Some(window) = working.window_override.as_ref() {
            self.window_override_enabled = true;
            self.window_start = window.start_rfc3339.clone();
            self.window_end = window.end_rfc3339.clone();
        } else {
            self.window_override_enabled = false;
            self.window_start.clear();
            self.window_end.clear();
        }

        self.english_override_enabled = working.english_only_override.is_some();
        self.english_override_value = working
            .english_only_override
            .unwrap_or(self.default_english);

        self.captions_override_enabled = working.require_captions_override.is_some();
        self.captions_override_value = working
            .require_captions_override
            .unwrap_or(self.default_captions);

        self.min_duration_override_enabled = working.min_duration_override.is_some();
        self.min_duration_override_value = working
            .min_duration_override
            .unwrap_or(self.default_min_duration);

        self.priority = working.priority;
        self.error = None;
        self.awaiting_clipboard = false;
        self.pending_clipboard = None;
        self.show_dirty_warning = false;
    }
}

#[derive(Debug, Clone)]
pub enum ImportMode {
    Clipboard,
    File,
}

#[derive(Debug, Clone)]
pub enum ExportMode {
    Clipboard,
    File,
}

pub struct ImportDialogState {
    pub raw_json: String,
    pub file_path: Option<String>,
    pub manual_path: String,
    pub mode: ImportMode,
    pub error: Option<String>,
    pub replace_existing: bool,
}

pub struct ExportDialogState {
    pub raw_json: String,
    pub file_path: Option<String>,
    pub manual_path: String,
    pub mode: ExportMode,
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

    fn sanitize_id_source(name: &str) -> String {
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
        base.trim_matches('-').to_string()
    }

    fn generate_unique_id_with(&self, name: &str, existing: &[MySearch]) -> String {
        let mut base = Self::sanitize_id_source(name);
        if base.is_empty() {
            base = format!("preset-{}", OffsetDateTime::now_utc().unix_timestamp());
        }
        let mut candidate = base.clone();
        let mut counter = 2usize;
        while existing.iter().any(|s| s.id == candidate) {
            candidate = format!("{}-{}", base, counter);
            counter += 1;
        }
        candidate
    }

    fn generate_unique_id(&self, name: &str) -> String {
        self.generate_unique_id_with(name, &self.prefs.searches)
    }

    pub(crate) fn parse_clipboard_preset(&self, raw: &str) -> anyhow::Result<MySearch> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("Clipboard is empty.");
        }

        if let Ok(preset) = serde_json::from_str::<MySearch>(trimmed) {
            return Ok(preset);
        }

        if let Ok(presets) = serde_json::from_str::<Vec<MySearch>>(trimmed) {
            if let Some(first) = presets.into_iter().next() {
                return Ok(first);
            }
        }

        if let Ok(payload) = serde_json::from_str::<Prefs>(trimmed) {
            if let Some(first) = payload.searches.into_iter().next() {
                return Ok(first);
            }
        }

        bail!("Clipboard JSON did not contain a preset.");
    }

    pub(crate) fn apply_clipboard_preset(&mut self, mut preset: MySearch) {
        if let Some(editor) = self.preset_editor.as_mut() {
            match editor.mode {
                PresetEditorMode::Edit { .. } => {
                    preset.id = editor.working.id.clone();
                }
                PresetEditorMode::Duplicate { .. } | PresetEditorMode::New => {
                    preset.id.clear();
                    preset.enabled = true;
                }
            }
            editor.apply_source(&preset);
        }
    }

    pub fn open_import_dialog(&mut self) {
        self.import_dialog = Some(ImportDialogState {
            raw_json: String::new(),
            file_path: None,
            manual_path: String::new(),
            mode: ImportMode::Clipboard,
            error: None,
            replace_existing: false,
        });
    }

    pub fn open_export_dialog(&mut self) {
        match serde_json::to_string_pretty(&self.prefs.searches) {
            Ok(raw_json) => {
                self.export_dialog = Some(ExportDialogState {
                    raw_json,
                    file_path: None,
                    manual_path: String::new(),
                    mode: ExportMode::Clipboard,
                });
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

    pub fn import_from_file(&mut self) {
        match native_dialog::FileDialog::new()
            .add_filter("JSON files", &["json"])
            .add_filter("All files", &["*"])
            .show_open_single_file()
        {
            Ok(Some(path)) => match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.import_dialog = Some(ImportDialogState {
                        raw_json: content,
                        file_path: Some(path.to_string_lossy().to_string()),
                        manual_path: path.to_string_lossy().to_string(),
                        mode: ImportMode::File,
                        error: None,
                        replace_existing: true,
                    });
                }
                Err(err) => {
                    self.status = format!("Failed to read file: {err}");
                }
            },
            Ok(None) => {
                // User cancelled - do nothing
            }
            Err(err) => {
                self.status = format!("Failed to open file dialog: {err}");
            }
        }
    }

    pub fn export_to_file(&mut self) {
        if let Some(dialog) = self.export_dialog.as_ref() {
            match native_dialog::FileDialog::new()
                .add_filter("JSON files", &["json"])
                .set_filename("yts_search_presets.json")
                .show_save_single_file()
            {
                Ok(Some(path)) => match std::fs::write(&path, &dialog.raw_json) {
                    Ok(_) => {
                        self.status = format!("Presets saved to: {}", path.display());
                        self.cancel_export_dialog();
                    }
                    Err(err) => {
                        self.status = format!("Failed to save file: {err}");
                    }
                },
                Ok(None) => {
                    // User cancelled - do nothing
                }
                Err(err) => {
                    self.status = format!("Failed to open save dialog: {err}");
                }
            }
        }
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
        if dialog.replace_existing {
            let mut new_list: Vec<MySearch> = Vec::new();
            for mut preset in presets {
                if preset.name.is_empty() {
                    continue;
                }
                let trimmed_id = preset.id.trim();
                if trimmed_id.is_empty() || new_list.iter().any(|s| s.id == trimmed_id) {
                    preset.id = self.generate_unique_id_with(&preset.name, &new_list);
                } else {
                    preset.id = trimmed_id.to_string();
                }
                new_list.push(preset);
            }
            if new_list.is_empty() {
                dialog.error = Some("No valid presets to import.".into());
                self.import_dialog = Some(dialog);
                return;
            }
            added = new_list.len();
            self.prefs.searches = new_list;
        } else {
            for mut preset in presets {
                if preset.name.is_empty() {
                    continue;
                }
                if preset.id.trim().is_empty()
                    || self.prefs.searches.iter().any(|s| s.id == preset.id)
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
        }

        if let Err(err) = prefs::save(&self.prefs) {
            dialog.error = Some(format!("Failed to save prefs: {err}"));
            self.import_dialog = Some(dialog);
            return;
        }

        self.status = format!("Imported {added} preset(s).");
        if dialog.replace_existing {
            self.selected_search_id = self.prefs.searches.first().map(|s| s.id.clone());
        } else {
            self.selected_search_id = self.prefs.searches.last().map(|s| s.id.clone());
        }
    }
}
