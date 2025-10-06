use egui::{Color32, Context, Frame, Margin, RichText};

use crate::prefs;
use crate::ui::theme::{ACCENT_EXTRA, ACCENT_OPEN, ACCENT_SAVE, PANEL_FILL};

use super::AppState;

enum PresetAction {
    Edit(usize),
    Duplicate(usize),
    Delete(usize),
}

pub(super) fn render(state: &mut AppState, ctx: &Context) {
    egui::SidePanel::left("left")
        .resizable(true)
        .show(ctx, |ui| {
            Frame::default()
                .fill(PANEL_FILL)
                .inner_margin(Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    let mut pending_action: Option<PresetAction> = None;

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |scroll_ui| {
                            scroll_ui.heading("My Searches");
                            scroll_ui.separator();
                            scroll_ui.label("API key:");
                            scroll_ui.text_edit_singleline(&mut state.prefs.api_key);
                            scroll_ui.add_space(8.0);
                            scroll_ui.horizontal(|ui| {
                                let new_button = egui::Button::new(
                                    RichText::new("New preset").strong().color(Color32::WHITE),
                                )
                                .fill(ACCENT_EXTRA)
                                .min_size(egui::vec2(120.0, 28.0));
                                if ui
                                    .add(new_button)
                                    .on_hover_text("Create a blank preset")
                                    .clicked()
                                {
                                    state.open_new_preset();
                                }

                                let import_button = egui::Button::new(
                                    RichText::new("Load presets").strong().color(Color32::WHITE),
                                )
                                .fill(ACCENT_SAVE)
                                .min_size(egui::vec2(120.0, 28.0));
                                if ui
                                    .add(import_button)
                                    .on_hover_text("Import presets from file")
                                    .clicked()
                                {
                                    state.open_import_dialog();
                                }

                                let export_button = egui::Button::new(
                                    RichText::new("Export presets")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(ACCENT_OPEN)
                                .min_size(egui::vec2(120.0, 28.0));
                                if ui
                                    .add(export_button)
                                    .on_hover_text("Save presets to file")
                                    .clicked()
                                {
                                    state.open_export_dialog();
                                }
                            });
                            scroll_ui.add_space(8.0);
                            let reset_button = egui::Button::new(
                                RichText::new("Reset defaults")
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(200, 60, 60))
                            .min_size(egui::vec2(140.0, 28.0));
                            let reset_response = scroll_ui.add(reset_button).on_hover_text(
                                "Restore built-in presets, clear blocks, and reset filters",
                            );
                            if reset_response.clicked() {
                                state.reset_to_defaults();
                            }
                            scroll_ui.add_space(8.0);
                            scroll_ui.label("Presets (enable/disable):");

                            let len = state.prefs.searches.len();
                            let mut any_enabled_changed = false;
                            for index in 0..len {
                                if let Some(search) = state.prefs.searches.get_mut(index) {
                                    let mut select_id: Option<String> = None;
                                    let mut row_action: Option<PresetAction> = None;
                                    scroll_ui.horizontal(|ui| {
                                        let old_enabled = search.enabled;
                                        ui.checkbox(&mut search.enabled, "");
                                        if old_enabled != search.enabled {
                                            any_enabled_changed = true;
                                        }
                                        let selected = state
                                            .selected_search_id
                                            .as_deref()
                                            .map(|id| id == search.id)
                                            .unwrap_or(false);
                                        if ui.selectable_label(selected, &search.name).clicked() {
                                            if selected {
                                                select_id = Some(String::new());
                                            } else {
                                                select_id = Some(search.id.clone());
                                            }
                                        }
                                        ui.menu_button("⋮", |menu_ui| {
                                            if !search.system {
                                                if menu_ui
                                                    .button("Edit")
                                                    .on_hover_text("Edit this preset")
                                                    .clicked()
                                                {
                                                    row_action = Some(PresetAction::Edit(index));
                                                    menu_ui.close_menu();
                                                }
                                            }
                                            if menu_ui
                                                .button("Duplicate")
                                                .on_hover_text("Copy this preset")
                                                .clicked()
                                            {
                                                row_action = Some(PresetAction::Duplicate(index));
                                                menu_ui.close_menu();
                                            }
                                            if !search.system {
                                                if menu_ui
                                                    .button("Delete")
                                                    .on_hover_text("Remove this preset")
                                                    .clicked()
                                                {
                                                    row_action = Some(PresetAction::Delete(index));
                                                    menu_ui.close_menu();
                                                }
                                            }
                                        });
                                    });
                                    if let Some(id) = select_id {
                                        if id.is_empty() {
                                            state.selected_search_id = None;
                                        } else {
                                            state.selected_search_id = Some(id);
                                        }
                                        state.refresh_visible_results();
                                    }
                                    if pending_action.is_none() {
                                        if let Some(action) = row_action {
                                            pending_action = Some(action);
                                        }
                                    }
                                }
                            }
                            if any_enabled_changed {
                                state.refresh_visible_results();
                            }

                            scroll_ui.add_space(8.0);
                            let save_button = egui::Button::new(
                                RichText::new("Save presets").strong().color(Color32::WHITE),
                            )
                            .fill(ACCENT_SAVE)
                            .min_size(egui::vec2(120.0, 28.0));
                            if scroll_ui
                                .add(save_button)
                                .on_hover_text("Persist presets to disk")
                                .clicked()
                            {
                                state.normalize_duration_selection();
                                if let Err(e) = prefs::save(&state.prefs) {
                                    state.status = format!("Save error: {e}");
                                } else {
                                    state.status = "Presets saved.".into();
                                }
                            }
                            scroll_ui.add_space(12.0);
                            scroll_ui.separator();
                            scroll_ui.add_space(12.0);
                            scroll_ui.label("Blocked channels:");
                            if state.prefs.blocked_channels.is_empty() {
                                scroll_ui.label("(none)");
                            } else {
                                let blocked_snapshot = state.prefs.blocked_channels.clone();
                                for entry in blocked_snapshot {
                                    let (key, label) = prefs::parse_block_entry(&entry);
                                    if key.is_empty() {
                                        continue;
                                    }
                                    scroll_ui.horizontal(|ui| {
                                        ui.label(label);
                                        if ui
                                            .button("Unblock")
                                            .on_hover_text("Allow videos from this channel again")
                                            .clicked()
                                        {
                                            state.unblock_channel(&key);
                                        }
                                    });
                                }
                            }
                        });

                    if let Some(action) = pending_action {
                        match action {
                            PresetAction::Edit(idx) => state.open_edit_preset(idx),
                            PresetAction::Duplicate(idx) => state.open_duplicate_preset(idx),
                            PresetAction::Delete(idx) => state.delete_preset(idx),
                        }
                    }
                });
        });
}
