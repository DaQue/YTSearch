use egui::{Align, Color32, Context, Frame, Layout, Margin, RichText};

use crate::prefs::TimeWindowPreset;
use crate::ui::theme::{
    ACCENT_ANY, ACCENT_SEARCH, ACCENT_SINGLE, PANEL_FILL, PRESET_COLORS, STATUS_ACCENT,
    tinted_toggle_button,
};
use crate::ui::utils::time_window_label;

use super::AppState;

pub(super) fn render(state: &mut AppState, ctx: &Context) -> bool {
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
                                RichText::new("YTSearch").color(Color32::from_rgb(229, 231, 235)),
                            );
                            ui.add_space(12.0);
                            ui.colored_label(STATUS_ACCENT, RichText::new(&state.status).strong());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Help").clicked() {
                                    state.show_help_dialog = true;
                                }
                                ui.add_space(6.0);
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
                            if tinted_toggle_button(ui, state.run_any_mode, "Any", ACCENT_ANY) {
                                state.run_any_mode = true;
                            }
                            ui.add_space(6.0);
                            if tinted_toggle_button(
                                ui,
                                !state.run_any_mode,
                                "Single",
                                ACCENT_SINGLE,
                            ) {
                                state.run_any_mode = false;
                            }
                            if !state.run_any_mode {
                                if let Some(name) = state.selected_search_name() {
                                    ui.add_space(10.0);
                                    ui.label(format!("Selected: {}", name));
                                }
                            }
                            ui.add_space(12.0);
                            egui::ComboBox::from_label("Date window")
                                .selected_text(time_window_label(state.prefs.global.default_window))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.prefs.global.default_window,
                                        TimeWindowPreset::Today,
                                        "Today",
                                    );
                                    ui.selectable_value(
                                        &mut state.prefs.global.default_window,
                                        TimeWindowPreset::H48,
                                        "48h",
                                    );
                                    ui.selectable_value(
                                        &mut state.prefs.global.default_window,
                                        TimeWindowPreset::D7,
                                        "7d",
                                    );
                                    ui.selectable_value(
                                        &mut state.prefs.global.default_window,
                                        TimeWindowPreset::AllTime,
                                        "Any date",
                                    );
                                });
                            ui.add_space(12.0);
                            ui.checkbox(&mut state.prefs.global.english_only, "English only");
                            ui.checkbox(
                                &mut state.prefs.global.require_captions,
                                "Require captions",
                            );
                            ui.label("Min duration (s):");
                            ui.add(
                                egui::DragValue::new(&mut state.prefs.global.min_duration_secs)
                                    .range(0..=7200),
                            );
                        });
                        ui.add_space(6.0);
                        let length_buttons: Vec<(String, String, bool, Color32)> = state
                            .duration_filter
                            .buckets
                            .iter()
                            .enumerate()
                            .map(|(idx, bucket)| {
                                let color = PRESET_COLORS[idx % PRESET_COLORS.len()];
                                let label = if bucket.selected {
                                    format!("● {}", bucket.config.label)
                                } else {
                                    bucket.config.label.clone()
                                };
                                (bucket.config.id.clone(), label, bucket.selected, color)
                            })
                            .collect();
                        if !length_buttons.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Length:");
                                ui.add_space(4.0);
                                for (id, label, selected, color) in length_buttons {
                                    if tinted_toggle_button(ui, selected, label.as_str(), color)
                                        && state.duration_filter.toggle(&id)
                                    {
                                        state.normalize_duration_selection();
                                    }
                                    ui.add_space(4.0);
                                }
                            });
                        }
                    });
                });
        });

    search_requested
}
