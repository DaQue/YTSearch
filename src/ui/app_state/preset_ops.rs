use anyhow::{Result as AnyResult, bail};
use serde_json;
use time::OffsetDateTime;

use crate::prefs::{self, MySearch, Prefs};

use super::{AppState, PresetEditorMode, PresetEditorState};

impl AppState {
    pub fn open_new_preset(&mut self) {
        let template = MySearch {
            priority: self.prefs.searches.len() as i32,
            ..MySearch::default()
        };
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

        let has_query_text = editor
            .working
            .query
            .q
            .as_ref()
            .map(|q| !q.trim().is_empty())
            .unwrap_or(false);
        if !has_query_text
            && editor.working.query.any_terms.is_empty()
            && editor.working.query.all_terms.is_empty()
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
                SaveAction::Update {
                    index,
                    id,
                    preset: editor.working.clone(),
                }
            }
            PresetEditorMode::Duplicate { .. } | PresetEditorMode::New => {
                if editor.working.id.trim().is_empty()
                    || self
                        .prefs
                        .searches
                        .iter()
                        .any(|preset| preset.id == editor.working.id)
                {
                    editor.working.id = self.generate_unique_id(&editor.working.name);
                }
                SaveAction::Append {
                    preset: editor.working.clone(),
                }
            }
        };

        match action {
            SaveAction::Update { index, id, preset } => {
                if let Some(existing) = self.prefs.searches.get_mut(index) {
                    *existing = preset;
                    existing.id = id;
                }
            }
            SaveAction::Append { preset } => {
                self.prefs.searches.push(preset);
            }
        }

        if let Err(err) = prefs::save(&self.prefs) {
            self.status = format!("Failed to save prefs: {err}");
        } else {
            self.status = "Preset saved.".into();
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

    pub(crate) fn generate_unique_id_with(&self, name: &str, existing: &[MySearch]) -> String {
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

    pub(crate) fn generate_unique_id(&self, name: &str) -> String {
        self.generate_unique_id_with(name, &self.prefs.searches)
    }

    pub(crate) fn parse_clipboard_preset(&self, raw: &str) -> AnyResult<MySearch> {
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            bail!("Clipboard is empty");
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
}
