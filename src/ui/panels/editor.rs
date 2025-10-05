use egui::{Align, Color32, Context, Layout, RichText};

use crate::prefs::MySearch;
use crate::ui::preset_editor::PresetEditorMode;
use crate::ui::theme::ACCENT_SAVE;

use super::AppState;
use super::helpers::render_token_editor;

pub(super) fn render(state: &mut AppState, ctx: &Context) {
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

    if let Some(editor) = state.preset_editor.as_mut() {
        if editor.awaiting_clipboard {
            if let Some(text) = pasted_text.clone() {
                editor.awaiting_clipboard = false;
                pending_clipboard_text = Some(text);
            }
        }

        let title = match editor.mode {
            PresetEditorMode::New => "New preset",
            PresetEditorMode::Edit { .. } => "Edit preset",
            PresetEditorMode::Duplicate { .. } => "Duplicate preset",
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
                                    egui::DragValue::new(&mut editor.min_duration_override_value)
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
                            egui::Button::new(RichText::new("Save preset").color(Color32::WHITE))
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
        match state.parse_clipboard_preset(&text) {
            Ok(preset) => {
                if let Some(editor) = state.preset_editor.as_mut() {
                    if editor.is_dirty() {
                        editor.pending_clipboard = Some(preset);
                        editor.show_dirty_warning = true;
                    } else {
                        apply_from_clipboard = Some(preset);
                    }
                }
            }
            Err(err) => {
                if let Some(editor) = state.preset_editor.as_mut() {
                    editor.error = Some(format!("Clipboard import failed: {err}"));
                    editor.awaiting_clipboard = false;
                }
            }
        }
    }

    if let Some(editor) = state.preset_editor.as_mut() {
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
                                egui::Button::new(RichText::new("Replace").color(Color32::WHITE))
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
        if let Some(editor) = state.preset_editor.as_mut() {
            if let Some(preset) = editor.pending_clipboard.take() {
                editor.show_dirty_warning = false;
                apply_from_clipboard = Some(preset);
            }
        }
    } else if cancel_replace {
        if let Some(editor) = state.preset_editor.as_mut() {
            editor.pending_clipboard = None;
            editor.show_dirty_warning = false;
        }
    }

    if let Some(json) = copy_payload {
        ctx.copy_text(json);
        state.status = "Preset copied to clipboard.".into();
    } else if let Some(err) = copy_error {
        state.status = err;
    }

    if let Some(preset) = apply_from_clipboard {
        state.apply_clipboard_preset(preset);
        state.status = "Preset loaded from clipboard.".into();
    }

    if wants_save {
        state.try_save_editor();
    } else if wants_cancel {
        state.cancel_editor();
    }
}
