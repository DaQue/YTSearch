use std::collections::HashSet;

use crate::prefs::{MySearch, TimeWindow};

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

type TermBuckets = (
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
);

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

    fn apply_terms_to_self(&mut self) -> TermBuckets {
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
            target.window_override = Some(TimeWindow {
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
