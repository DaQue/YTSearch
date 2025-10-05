use egui::{Align, Color32, Context, RichText, TextEdit, TextStyle};

use crate::ui::app_state::ImportMode;
use crate::ui::theme::ACCENT_SAVE;

use super::AppState;

pub(super) fn render(state: &mut AppState, ctx: &Context) {
    render_import_dialog(state, ctx);
    render_export_dialog(state, ctx);
}

fn render_import_dialog(state: &mut AppState, ctx: &Context) {
    let mut wants_import = false;
    let mut wants_cancel_import = false;
    let mut wants_switch_to_file = false;
    let mut wants_switch_to_clipboard = false;

    if let Some(dialog) = state.import_dialog.as_mut() {
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
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
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
        state.apply_import();
    } else if wants_cancel_import {
        state.cancel_import_dialog();
    } else if wants_switch_to_file {
        state.import_from_file();
    } else if wants_switch_to_clipboard {
        if let Some(dialog) = state.import_dialog.as_mut() {
            dialog.mode = ImportMode::Clipboard;
            dialog.file_path = None;
            dialog.replace_existing = false;
        }
    }
}

fn render_export_dialog(state: &mut AppState, ctx: &Context) {
    let mut wants_close_export = false;
    let mut wants_switch_to_file_export = false;
    let mut wants_copy_to_clipboard = false;

    if let Some(dialog) = state.export_dialog.as_mut() {
        let mut open = true;
        egui::Window::new("Export presets")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.set_min_width(420.0);

                ui.horizontal(|ui| {
                    if ui.button("💾 Save to file").clicked() {
                        wants_switch_to_file_export = true;
                    }
                    if ui.button("📋 Copy to clipboard").clicked() {
                        wants_copy_to_clipboard = true;
                    }
                });

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
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
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
        state.cancel_export_dialog();
    } else if wants_switch_to_file_export {
        state.export_to_file();
    } else if wants_copy_to_clipboard {
        if let Some(dialog) = state.export_dialog.as_ref() {
            ctx.copy_text(dialog.raw_json.clone());
            state.status = "Export JSON copied.".into();
        }
    }
}
