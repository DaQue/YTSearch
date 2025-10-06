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
                                if ui
                                    .button("Help")
                                    .on_hover_text("Show in-app help and shortcuts")
                                    .clicked()
                                {
                                    state.show_help_dialog = true;
                                }
                                ui.add_space(6.0);
                                let search_button = egui::Button::new(
                                    RichText::new("Search").strong().color(Color32::WHITE),
                                )
                                .fill(ACCENT_SEARCH)
                                .min_size(egui::vec2(120.0, 32.0));
                                if ui
                                    .add(search_button)
                                    .on_hover_text(
                                        "Fetch results from YouTube with current filters",
                                    )
                                    .clicked()
                                {
                                    search_requested = true;
                                }
                            });
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let desired =
                                [(false, "Single", ACCENT_SINGLE), (true, "Any", ACCENT_ANY)];
                            let previous = state.run_any_mode;
                            for (idx, (is_any, label, color)) in desired.iter().enumerate() {
                                let active = state.run_any_mode == *is_any;
                                let fill = if active {
                                    *color
                                } else {
                                    color.linear_multiply(0.25)
                                };
                                let text_color = if active { Color32::WHITE } else { *color };
                                let response = ui
                                    .add_sized(
                                        egui::vec2(88.0, 28.0),
                                        egui::Button::new(RichText::new(*label).color(text_color))
                                            .fill(fill)
                                            .corner_radius(6.0),
                                    )
                                    .on_hover_text(match is_any {
                                        true => "Run every enabled preset",
                                        false => "Run only the selected preset",
                                    });
                                if response.clicked() {
                                    state.run_any_mode = *is_any;
                                }
                                if idx == 0 {
                                    ui.add_space(4.0);
                                }
                            }
                            if previous != state.run_any_mode {
                                state.refresh_visible_results();
                            }
                            if state.run_any_mode {
                                let enabled = state
                                    .prefs
                                    .searches
                                    .iter()
                                    .filter(|preset| preset.enabled)
                                    .count();
                                ui.add_space(8.0);
                                ui.label(format!(
                                    "{} preset{} enabled",
                                    enabled,
                                    if enabled == 1 { "" } else { "s" }
                                ));
                            } else if let Some(name) = state.selected_search_name() {
                                ui.add_space(8.0);
                                ui.label(format!("Single: {}", name));
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
                            let old_english_only = state.prefs.global.english_only;
                            ui.checkbox(&mut state.prefs.global.english_only, "English only");
                            if old_english_only != state.prefs.global.english_only {
                                state.refresh_visible_results();
                            }
                            let old_require_captions = state.prefs.global.require_captions;
                            ui.checkbox(
                                &mut state.prefs.global.require_captions,
                                "Require captions",
                            );
                            if old_require_captions != state.prefs.global.require_captions {
                                state.refresh_visible_results();
                            }
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
                                        state.refresh_visible_results();
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
