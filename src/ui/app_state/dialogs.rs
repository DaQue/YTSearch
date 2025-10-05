use crate::prefs::{self, MySearch, Prefs};

use super::AppState;
use serde_json;

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

impl AppState {
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
            Ok(None) => {}
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
                Ok(None) => {}
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
        let mut presets = match serde_json::from_str::<Vec<MySearch>>(&dialog.raw_json) {
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
