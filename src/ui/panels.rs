use crate::prefs::{self, MySearch, TimeWindowPreset};
use crate::yt::types::VideoDetails;
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Key, Layout, Margin, RichText, Stroke, TextEdit,
    TextStyle,
};

use super::app_state::AppState;
use super::theme::*;
use super::utils::{open_in_browser, time_window_label};

impl AppState {
    pub fn render_top_panel(&mut self, ctx: &Context) -> bool {
        let mut search_requested = false;

        egui::TopBottomPanel::top("top")
            .resizable(false)
            .show(ctx, |ui| {
                Frame::default()
                    .fill(PANEL_FILL)
                    .inner_margin(Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(
                                    RichText::new("YTSearch")
                                        .color(Color32::from_rgb(229, 231, 235)),
                                );
                                ui.add_space(12.0);
                                ui.colored_label(
                                    STATUS_ACCENT,
                                    RichText::new(&self.status).strong(),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let search_button = egui::Button::new(
                                        RichText::new("Search").strong().color(Color32::WHITE),
                                    )
                                    .fill(ACCENT_SEARCH)
                                    .min_size(egui::vec2(120.0, 32.0));
                                    if ui.add(search_button).clicked() {
                                        search_requested = true;
                                    }
                                });
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if tinted_toggle_button(ui, self.run_any_mode, "Any", ACCENT_ANY) {
                                    self.run_any_mode = true;
                                }
                                ui.add_space(6.0);
                                if tinted_toggle_button(
                                    ui,
                                    !self.run_any_mode,
                                    "Single",
                                    ACCENT_SINGLE,
                                ) {
                                    self.run_any_mode = false;
                                }
                                if !self.run_any_mode {
                                    if let Some(name) = self.selected_search_name() {
                                        ui.add_space(10.0);
                                        ui.label(format!("Selected: {}", name));
                                    }
                                }
                                ui.add_space(12.0);
                                egui::ComboBox::from_label("Date window")
                                    .selected_text(time_window_label(
                                        self.prefs.global.default_window,
                                    ))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.prefs.global.default_window,
                                            TimeWindowPreset::Today,
                                            "Today",
                                        );
                                        ui.selectable_value(
                                            &mut self.prefs.global.default_window,
                                            TimeWindowPreset::H48,
                                            "48h",
                                        );
                                        ui.selectable_value(
                                            &mut self.prefs.global.default_window,
                                            TimeWindowPreset::D7,
                                            "7d",
                                        );
                                        ui.selectable_value(
                                            &mut self.prefs.global.default_window,
                                            TimeWindowPreset::Custom,
                                            "Custom",
                                        );
                                    });
                                ui.add_space(12.0);
                                ui.checkbox(&mut self.prefs.global.english_only, "English only");
                                ui.checkbox(
                                    &mut self.prefs.global.require_captions,
                                    "Require captions",
                                );
                                ui.label("Min duration (s):");
                                ui.add(
                                    egui::DragValue::new(&mut self.prefs.global.min_duration_secs)
                                        .range(0..=7200),
                                );
                            });
                        });
                    });
            });

        search_requested
    }

    pub fn render_left_panel(&mut self, ctx: &Context) {
        egui::SidePanel::left("left")
            .resizable(true)
            .show(ctx, |ui| {
                Frame::default()
                    .fill(PANEL_FILL)
                    .inner_margin(Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        enum PresetAction {
                            Edit(usize),
                            Duplicate(usize),
                            Delete(usize),
                        }

                        let mut pending_action: Option<PresetAction> = None;

                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |scroll_ui| {
                                scroll_ui.heading("My Searches");
                                scroll_ui.separator();
                                scroll_ui.label("API key:");
                                scroll_ui.text_edit_singleline(&mut self.prefs.api_key);
                                scroll_ui.add_space(8.0);
                                scroll_ui.horizontal(|ui| {
                                    let new_button = egui::Button::new(
                                        RichText::new("New preset").strong().color(Color32::WHITE),
                                    )
                                    .fill(ACCENT_EXTRA)
                                    .min_size(egui::vec2(120.0, 28.0));
                                    if ui.add(new_button).clicked() {
                                        self.open_new_preset();
                                    }

                                    let import_button = egui::Button::new(
                                        RichText::new("Load presets")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(ACCENT_SAVE)
                                    .min_size(egui::vec2(120.0, 28.0));
                                    if ui.add(import_button).clicked() {
                                        self.open_import_dialog();
                                    }

                                    let export_button = egui::Button::new(
                                        RichText::new("Export presets")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(ACCENT_OPEN)
                                    .min_size(egui::vec2(120.0, 28.0));
                                    if ui.add(export_button).clicked() {
                                        self.open_export_dialog();
                                    }
                                });
                                scroll_ui.add_space(8.0);
                                scroll_ui.label("Presets (enable/disable):");

                                let len = self.prefs.searches.len();
                                for index in 0..len {
                                    if let Some(search) = self.prefs.searches.get_mut(index) {
                                        let mut select_id: Option<String> = None;
                                        let mut row_action: Option<PresetAction> = None;
                                        scroll_ui.horizontal(|ui| {
                                            ui.checkbox(&mut search.enabled, "");
                                            let selected = self
                                                .selected_search_id
                                                .as_deref()
                                                .map(|id| id == search.id)
                                                .unwrap_or(false);
                                            if ui.selectable_label(selected, &search.name).clicked()
                                            {
                                                select_id = Some(search.id.clone());
                                            }
                                            ui.menu_button("⋮", |menu_ui| {
                                                if menu_ui.button("Edit").clicked() {
                                                    row_action = Some(PresetAction::Edit(index));
                                                    menu_ui.close_menu();
                                                }
                                                if menu_ui.button("Duplicate").clicked() {
                                                    row_action =
                                                        Some(PresetAction::Duplicate(index));
                                                    menu_ui.close_menu();
                                                }
                                                if menu_ui.button("Delete").clicked() {
                                                    row_action = Some(PresetAction::Delete(index));
                                                    menu_ui.close_menu();
                                                }
                                            });
                                        });
                                        if let Some(id) = select_id {
                                            self.selected_search_id = Some(id);
                                        }
                                        if pending_action.is_none() {
                                            if let Some(action) = row_action {
                                                pending_action = Some(action);
                                            }
                                        }
                                    }
                                }

                                scroll_ui.add_space(8.0);
                                let save_button = egui::Button::new(
                                    RichText::new("Save presets").strong().color(Color32::WHITE),
                                )
                                .fill(ACCENT_SAVE)
                                .min_size(egui::vec2(120.0, 28.0));
                                if scroll_ui.add(save_button).clicked() {
                                    if let Err(e) = prefs::save(&self.prefs) {
                                        self.status = format!("Save error: {e}");
                                    } else {
                                        self.status = "Presets saved.".into();
                                    }
                                }
                                scroll_ui.add_space(12.0);
                                scroll_ui.separator();
                                scroll_ui.add_space(12.0);
                                scroll_ui.label("Blocked channels:");
                                if self.prefs.blocked_channels.is_empty() {
                                    scroll_ui.label("(none)");
                                } else {
                                    let blocked_snapshot = self.prefs.blocked_channels.clone();
                                    for entry in blocked_snapshot {
                                        let (key, label) = prefs::parse_block_entry(&entry);
                                        if key.is_empty() {
                                            continue;
                                        }
                                        scroll_ui.horizontal(|ui| {
                                            ui.label(label);
                                            if ui.button("Unblock").clicked() {
                                                self.unblock_channel(&key);
                                            }
                                        });
                                    }
                                }
                            });

                        if let Some(action) = pending_action {
                            match action {
                                PresetAction::Edit(idx) => self.open_edit_preset(idx),
                                PresetAction::Duplicate(idx) => self.open_duplicate_preset(idx),
                                PresetAction::Delete(idx) => self.delete_preset(idx),
                            }
                        }
                    });
            });
    }

    pub fn render_central_panel(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Results");
            if self.is_searching {
                ui.label("Searching...");
            } else if self.results.is_empty() {
                ui.label("No results yet. Enter your API key and click Search.");
            } else {
                let mut block_requests: Vec<(String, String)> = Vec::new();
                let results_snapshot = self.results.clone();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for video in &results_snapshot {
                        self.render_video_card(ui, video, &mut block_requests);
                        ui.add_space(6.0);
                    }
                });
                for (channel_id, channel_title) in block_requests {
                    self.block_channel(&channel_id, &channel_title);
                }
            }
        });
    }

    pub fn render_editor_window(&mut self, ctx: &Context) {
        let mut wants_save = false;
        let mut wants_cancel = false;

        let pasted_text = ctx.input(|i| {
            i.events.iter().rev().find_map(|event| match event {
                egui::Event::Paste(text) => Some(text.clone()),
                _ => None,
            })
        });

        let mut copy_payload: Option<String> = None;
        let mut copy_error: Option<String> = None;
        let mut pending_clipboard_text: Option<String> = None;
        let mut apply_from_clipboard: Option<MySearch> = None;
        let mut confirm_replace = false;
        let mut cancel_replace = false;

        if let Some(editor) = self.preset_editor.as_mut() {
            if editor.awaiting_clipboard {
                if let Some(text) = pasted_text.clone() {
                    editor.awaiting_clipboard = false;
                    pending_clipboard_text = Some(text);
                }
            }

            let title = match editor.mode {
                super::app_state::PresetEditorMode::New => "New preset",
                super::app_state::PresetEditorMode::Edit { .. } => "Edit preset",
                super::app_state::PresetEditorMode::Duplicate { .. } => "Duplicate preset",
            };

            let mut open = true;
            egui::Window::new(title)
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.set_min_width(420.0);
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.checkbox(&mut editor.enabled, "Enabled");
                            ui.label("Name");
                            ui.text_edit_singleline(&mut editor.name);

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy preset").clicked() {
                                    let snapshot = editor.snapshot();
                                    match serde_json::to_string_pretty(&snapshot) {
                                        Ok(json) => copy_payload = Some(json),
                                        Err(err) => {
                                            copy_error =
                                                Some(format!("Failed to serialize preset: {err}"));
                                        }
                                    }
                                }
                                if ui.button("📥 Paste preset").clicked() {
                                    editor.awaiting_clipboard = true;
                                    editor.error = None;
                                    editor.pending_clipboard = None;
                                    editor.show_dirty_warning = false;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                                }
                                if editor.awaiting_clipboard {
                                    ui.label("Waiting for clipboard…");
                                }
                            });

                            ui.separator();
                            ui.label("Free-text query");
                            ui.text_edit_singleline(&mut editor.query_text);

                            ui.add_space(6.0);
                            render_token_editor(
                                ui,
                                "Any terms (OR match)",
                                &mut editor.any_terms,
                                &mut editor.new_any_term,
                                "Add term",
                            );

                            ui.add_space(6.0);
                            render_token_editor(
                                ui,
                                "All terms (AND match)",
                                &mut editor.all_terms,
                                &mut editor.new_all_term,
                                "Add required term",
                            );

                            ui.add_space(6.0);
                            render_token_editor(
                                ui,
                                "Not terms (exclude)",
                                &mut editor.not_terms,
                                &mut editor.new_not_term,
                                "Add excluded term",
                            );

                            ui.add_space(6.0);
                            render_token_editor(
                                ui,
                                "Allowed channels (handles or IDs)",
                                &mut editor.channel_allow,
                                &mut editor.new_allow_entry,
                                "Add allowed channel",
                            );

                            ui.add_space(6.0);
                            render_token_editor(
                                ui,
                                "Blocked channels (handles or IDs)",
                                &mut editor.channel_deny,
                                &mut editor.new_deny_entry,
                                "Add blocked channel",
                            );

                            ui.separator();
                            if ui
                                .checkbox(
                                    &mut editor.window_override_enabled,
                                    "Override time window (RFC3339)",
                                )
                                .clicked()
                            {
                                if !editor.window_override_enabled {
                                    editor.window_start.clear();
                                    editor.window_end.clear();
                                }
                            }
                            if editor.window_override_enabled {
                                ui.label("Start");
                                ui.text_edit_singleline(&mut editor.window_start);
                                ui.label("End");
                                ui.text_edit_singleline(&mut editor.window_end);
                            }

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut editor.english_override_enabled,
                                    "Override English-only",
                                );
                                ui.add_enabled_ui(editor.english_override_enabled, |ui| {
                                    ui.selectable_value(
                                        &mut editor.english_override_value,
                                        true,
                                        "Require English",
                                    );
                                    ui.selectable_value(
                                        &mut editor.english_override_value,
                                        false,
                                        "Allow any language",
                                    );
                                });
                            });

                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut editor.captions_override_enabled,
                                    "Override 'Require captions'",
                                );
                                ui.add_enabled_ui(editor.captions_override_enabled, |ui| {
                                    ui.selectable_value(
                                        &mut editor.captions_override_value,
                                        true,
                                        "Require captions",
                                    );
                                    ui.selectable_value(
                                        &mut editor.captions_override_value,
                                        false,
                                        "Captions optional",
                                    );
                                });
                            });

                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut editor.min_duration_override_enabled,
                                    "Override min duration (seconds)",
                                );
                                ui.add_enabled_ui(editor.min_duration_override_enabled, |ui| {
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut editor.min_duration_override_value,
                                        )
                                        .range(0..=7200),
                                    );
                                });
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Priority (Any mode sort, higher first)");
                                ui.add(egui::DragValue::new(&mut editor.priority).speed(1));
                            });
                        });

                    if let Some(err) = editor.error.as_ref() {
                        ui.add_space(6.0);
                        ui.colored_label(Color32::from_rgb(239, 68, 68), err);
                    }

                    ui.add_space(10.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Save preset").color(Color32::WHITE),
                                )
                                .fill(ACCENT_SAVE),
                            )
                            .clicked()
                        {
                            wants_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            wants_cancel = true;
                        }
                    });
                });

            if !open {
                wants_cancel = true;
            }
        }

        if let Some(text) = pending_clipboard_text {
            match self.parse_clipboard_preset(&text) {
                Ok(preset) => {
                    if let Some(editor) = self.preset_editor.as_mut() {
                        if editor.is_dirty() {
                            editor.pending_clipboard = Some(preset);
                            editor.show_dirty_warning = true;
                        } else {
                            apply_from_clipboard = Some(preset);
                        }
                    }
                }
                Err(err) => {
                    if let Some(editor) = self.preset_editor.as_mut() {
                        editor.error = Some(format!("Clipboard import failed: {err}"));
                        editor.awaiting_clipboard = false;
                    }
                }
            }
        }

        if let Some(editor) = self.preset_editor.as_mut() {
            if editor.show_dirty_warning {
                let mut open_confirm = true;
                egui::Window::new("Unsaved changes")
                    .open(&mut open_confirm)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, -40.0))
                    .show(ctx, |ui| {
                        ui.label("You have unsaved edits. Replace them with the clipboard preset?");
                        ui.add_space(10.0);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Replace").color(Color32::WHITE),
                                    )
                                    .fill(ACCENT_SAVE),
                                )
                                .clicked()
                            {
                                confirm_replace = true;
                            }
                            if ui.button("Keep current").clicked() {
                                cancel_replace = true;
                            }
                        });
                    });
                if !open_confirm {
                    cancel_replace = true;
                }
            }
        }

        if confirm_replace {
            if let Some(editor) = self.preset_editor.as_mut() {
                if let Some(preset) = editor.pending_clipboard.take() {
                    editor.show_dirty_warning = false;
                    apply_from_clipboard = Some(preset);
                }
            }
        } else if cancel_replace {
            if let Some(editor) = self.preset_editor.as_mut() {
                editor.pending_clipboard = None;
                editor.show_dirty_warning = false;
            }
        }

        if let Some(json) = copy_payload {
            ctx.copy_text(json);
            self.status = "Preset copied to clipboard.".into();
        } else if let Some(err) = copy_error {
            self.status = err;
        }

        if let Some(preset) = apply_from_clipboard {
            self.apply_clipboard_preset(preset);
            self.status = "Preset loaded from clipboard.".into();
        }

        if wants_save {
            self.try_save_editor();
        } else if wants_cancel {
            self.cancel_editor();
        }
    }

    pub fn render_import_export_windows(&mut self, ctx: &Context) {
        let mut wants_import = false;
        let mut wants_cancel_import = false;
        let mut wants_switch_to_file = false;
        let mut wants_switch_to_clipboard = false;

        if let Some(dialog) = self.import_dialog.as_mut() {
            let mut open = true;
            egui::Window::new("Import presets")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.set_min_width(420.0);

                    // Mode selection buttons
                    ui.horizontal(|ui| {
                        if ui.button("📁 Load from file").clicked() {
                            wants_switch_to_file = true;
                        }
                        if ui.button("📋 Paste from clipboard").clicked() {
                            wants_switch_to_clipboard = true;
                        }
                    });

                    // Show current file path if loaded from file
                    if let Some(path) = &dialog.file_path {
                        ui.label(format!("Loaded from: {}", path));
                    }

                    ui.add_space(6.0);
                    ui.label("Paste a JSON array of presets or a prefs.json snippet.");
                    ui.checkbox(&mut dialog.replace_existing, "Replace existing presets");
                    ui.small("Checked: imported presets overwrite the current list. Unchecked: imported presets are added alongside existing ones.");
                    egui::ScrollArea::both()
                        .max_height(260.0)
                        .auto_shrink([false, false])
                        .show(ui, |scroll_ui| {
                            scroll_ui.add(
                                TextEdit::multiline(&mut dialog.raw_json)
                                    .code_editor()
                                    .desired_rows(16)
                                    .desired_width(520.0),
                            );
                        });
                    if let Some(err) = dialog.error.as_ref() {
                        ui.add_space(6.0);
                        ui.colored_label(Color32::from_rgb(239, 68, 68), err);
                    }
                    ui.add_space(10.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Import").color(Color32::WHITE))
                                    .fill(ACCENT_SAVE),
                            )
                            .clicked()
                        {
                            wants_import = true;
                        }
                        if ui.button("Cancel").clicked() {
                            wants_cancel_import = true;
                        }
                    });
                });
            if !open {
                wants_cancel_import = true;
            }
        }

        if wants_import {
            self.apply_import();
        } else if wants_cancel_import {
            self.cancel_import_dialog();
        } else if wants_switch_to_file {
            self.import_from_file();
        } else if wants_switch_to_clipboard {
            if let Some(dialog) = self.import_dialog.as_mut() {
                dialog.mode = super::app_state::ImportMode::Clipboard;
                dialog.file_path = None;
                dialog.replace_existing = false;
            }
        }

        let mut wants_close_export = false;
        let mut wants_switch_to_file_export = false;
        let mut wants_copy_to_clipboard = false;

        if let Some(dialog) = self.export_dialog.as_mut() {
            let mut open = true;
            egui::Window::new("Export presets")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.set_min_width(420.0);

                    // Mode selection buttons
                    ui.horizontal(|ui| {
                        if ui.button("💾 Save to file").clicked() {
                            wants_switch_to_file_export = true;
                        }
                        if ui.button("📋 Copy to clipboard").clicked() {
                            wants_copy_to_clipboard = true;
                        }
                    });

                    // File operations are handled directly by native dialogs

                    ui.add_space(6.0);
                    ui.label("Copy this JSON to share or back up your presets.");
                    egui::ScrollArea::both()
                        .max_height(260.0)
                        .auto_shrink([false, false])
                        .show(ui, |scroll_ui| {
                            scroll_ui.add(
                                TextEdit::multiline(&mut dialog.raw_json)
                                    .code_editor()
                                    .desired_rows(16)
                                    .desired_width(520.0)
                                    .font(TextStyle::Monospace),
                            );
                        });
                    ui.add_space(10.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            wants_close_export = true;
                        }
                    });
                });
            if !open {
                wants_close_export = true;
            }
        }

        if wants_close_export {
            self.cancel_export_dialog();
        } else if wants_switch_to_file_export {
            self.export_to_file();
        } else if wants_copy_to_clipboard {
            if let Some(dialog) = self.export_dialog.as_ref() {
                ctx.copy_text(dialog.raw_json.clone());
                self.status = "Export JSON copied.".into();
            }
        }
    }

    fn render_video_card(
        &mut self,
        ui: &mut egui::Ui,
        video: &VideoDetails,
        block_requests: &mut Vec<(String, String)>,
    ) {
        Frame::default()
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0, CARD_BORDER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let title = RichText::new(&video.title)
                        .heading()
                        .color(Color32::from_rgb(229, 231, 235));
                    ui.label(title);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let open_button =
                            egui::Button::new(RichText::new("Open").strong().color(Color32::WHITE))
                                .fill(ACCENT_OPEN)
                                .min_size(egui::vec2(90.0, 26.0));
                        if ui.add(open_button).clicked() {
                            match open_in_browser(&video.url) {
                                Ok(()) => {
                                    self.status = "Opened video in browser.".into();
                                }
                                Err(err) => {
                                    self.status = format!("Failed to open browser: {err}");
                                }
                            }
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let channel_label = channel_display_label(video);
                    ui.label(format!("Channel: {}", channel_label));
                    if self.is_channel_blocked(video) {
                        ui.label(RichText::new("Blocked").color(ACCENT_EXTRA).strong());
                    } else {
                        let block_button = egui::Button::new(
                            RichText::new("Block channel")
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(ACCENT_EXTRA)
                        .min_size(egui::vec2(140.0, 24.0));
                        if ui.add(block_button).clicked() {
                            block_requests.push((
                                video.channel_handle.trim().to_owned(),
                                channel_label.clone(),
                            ));
                        }
                    }
                });
                ui.label(format!("Published: {}", video.published_at));
                ui.label(format!("Duration: {} sec", video.duration_secs));
                if !video.source_presets.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Presets:");
                        for (idx, preset_name) in video.source_presets.iter().enumerate() {
                            let color = PRESET_COLORS[idx % PRESET_COLORS.len()];
                            let fill = color.linear_multiply(0.18);
                            let stroke = Stroke::new(1.0, color);
                            let text = RichText::new(preset_name).color(color);
                            Frame::default()
                                .fill(fill)
                                .stroke(stroke)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(6, 3))
                                .show(ui, |ui| {
                                    ui.label(text.clone());
                                });
                        }
                    });
                }
            });
    }
}

fn render_token_editor(
    ui: &mut egui::Ui,
    label: &str,
    tokens: &mut Vec<String>,
    new_token: &mut String,
    hint: &str,
) {
    ui.label(label);

    let mut removals: Vec<usize> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for (idx, token) in tokens.iter().enumerate() {
            let color = PRESET_COLORS[idx % PRESET_COLORS.len()];
            let fill = color.linear_multiply(0.15);
            let stroke = Stroke::new(1.0, color);
            Frame::default()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(token).color(color));
                        ui.add_space(6.0);
                        if ui.small_button("×").clicked() {
                            removals.push(idx);
                        }
                    });
                });
        }
    });

    if !removals.is_empty() {
        removals.sort_unstable();
        removals.drain(..).rev().for_each(|idx| {
            if idx < tokens.len() {
                tokens.remove(idx);
            }
        });
        crate::ui::app_state::PresetEditorState::normalize_terms(tokens);
    }

    ui.horizontal(|ui| {
        let response = ui.add(TextEdit::singleline(new_token).hint_text(hint));
        let mut commit = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if ui.button("Add").clicked() {
            commit = true;
        }

        if commit {
            let value = new_token.trim();
            if !value.is_empty() {
                if !tokens
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(value))
                {
                    tokens.push(value.to_string());
                    crate::ui::app_state::PresetEditorState::normalize_terms(tokens);
                }
            }
            new_token.clear();
        }
    });
}

fn channel_display_label(video: &VideoDetails) -> String {
    let preferred_name = video
        .channel_display_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            let trimmed = video.channel_title.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

    let handle = video
        .channel_custom_url
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    match (preferred_name, handle) {
        (Some(name), Some(handle)) => {
            if handle.eq_ignore_ascii_case(&name) {
                name
            } else {
                format!("{} ({})", name, handle)
            }
        }
        (Some(name), None) => name,
        (None, Some(handle)) => handle,
        (None, None) => {
            let trimmed = video.channel_handle.trim();
            if trimmed.is_empty() {
                "Unknown channel".to_string()
            } else {
                trimmed.to_string()
            }
        }
    }
}
