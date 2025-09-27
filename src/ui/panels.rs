use crate::prefs::{self, TimeWindowPreset};
use crate::yt::types::VideoDetails;
use egui::{Align, Color32, Context, CornerRadius, Frame, Layout, Margin, RichText, Stroke};

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
                        ui.heading("My Searches");
                        ui.separator();
                        ui.label("API key:");
                        ui.text_edit_singleline(&mut self.prefs.api_key);
                        ui.add_space(8.0);
                        let new_button = egui::Button::new(
                            RichText::new("New preset").strong().color(Color32::WHITE),
                        )
                        .fill(ACCENT_EXTRA)
                        .min_size(egui::vec2(120.0, 28.0));
                        if ui.add(new_button).clicked() {
                            self.status =
                                "Preset editor not implemented yet; coming soon.".to_string();
                        }
                        ui.add_space(8.0);
                        ui.label("Presets (enable/disable):");
                        for s in &mut self.prefs.searches {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut s.enabled, "");
                                let selected = self
                                    .selected_search_id
                                    .as_deref()
                                    .map(|id| id == s.id)
                                    .unwrap_or(false);
                                if ui.selectable_label(selected, &s.name).clicked() {
                                    self.selected_search_id = Some(s.id.clone());
                                }
                                if ui.small_button("⋮").clicked() {
                                    self.status =
                                        "Preset editor not implemented yet; coming soon.".into();
                                }
                            });
                        }
                        ui.add_space(8.0);
                        let save_button = egui::Button::new(
                            RichText::new("Save prefs").strong().color(Color32::WHITE),
                        )
                        .fill(ACCENT_SAVE)
                        .min_size(egui::vec2(120.0, 28.0));
                        if ui.add(save_button).clicked() {
                            if let Err(e) = prefs::save(&self.prefs) {
                                self.status = format!("Save error: {e}");
                            } else {
                                self.status = "Prefs saved.".into();
                            }
                        }
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                        ui.label("Blocked channels:");
                        if self.prefs.blocked_channels.is_empty() {
                            ui.label("(none)");
                        } else {
                            let blocked_snapshot = self.prefs.blocked_channels.clone();
                            for entry in blocked_snapshot {
                                let (key, label) = prefs::parse_block_entry(&entry);
                                if key.is_empty() {
                                    continue;
                                }
                                ui.horizontal(|ui| {
                                    ui.label(label);
                                    if ui.button("Unblock").clicked() {
                                        self.unblock_channel(&key);
                                    }
                                });
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
                    ui.label(format!("Channel: {}", video.channel_title));
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
                                video.channel_title.trim().to_owned(),
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
