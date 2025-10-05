use egui::{Align, Color32, Context, Frame, Margin, RichText, Stroke};

use crate::ui::panels::helpers::channel_display_label;
use crate::ui::theme::{ACCENT_EXTRA, ACCENT_OPEN, CARD_BG, CARD_BORDER, PRESET_COLORS};
use crate::ui::utils::{format_duration, open_in_browser};
use crate::yt::types::VideoDetails;

use super::AppState;
use crate::ui::app_state::ResultSort;

pub(super) fn render(state: &mut AppState, ctx: &Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Results");
            ui.add_space(8.0);
            let previous_sort = state.result_sort;
            egui::ComboBox::from_label("Sort")
                .selected_text(state.result_sort.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.result_sort, ResultSort::Newest, "Newest");
                    ui.selectable_value(&mut state.result_sort, ResultSort::Oldest, "Oldest");
                    ui.selectable_value(&mut state.result_sort, ResultSort::Shortest, "Shortest");
                    ui.selectable_value(&mut state.result_sort, ResultSort::Longest, "Longest");
                    ui.selectable_value(&mut state.result_sort, ResultSort::Channel, "Channel");
                });
            if state.result_sort != previous_sort {
                state.apply_result_sort();
            }
        });
        if state.is_searching {
            ui.label("Searching...");
        } else if state.results.is_empty() {
            ui.label("No results yet. Enter your API key and click Search.");
        } else {
            let mut block_requests: Vec<(String, String)> = Vec::new();
            let results_snapshot = state.results.clone();
            let filtered_results: Vec<VideoDetails> = results_snapshot
                .into_iter()
                .filter(|video| state.duration_filter.allows(video.duration_secs))
                .collect();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for video in &filtered_results {
                    render_video_card(state, ui, video, &mut block_requests);
                    ui.add_space(6.0);
                }
            });
            for (channel_id, channel_title) in block_requests {
                state.block_channel(&channel_id, &channel_title);
            }
        }
    });
}

fn render_video_card(
    state: &mut AppState,
    ui: &mut egui::Ui,
    video: &VideoDetails,
    block_requests: &mut Vec<(String, String)>,
) {
    Frame::default()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, CARD_BORDER))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let title = RichText::new(&video.title)
                    .heading()
                    .color(Color32::from_rgb(229, 231, 235));
                ui.label(title);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let open_button =
                        egui::Button::new(RichText::new("Open").strong().color(Color32::WHITE))
                            .fill(ACCENT_OPEN)
                            .min_size(egui::vec2(90.0, 26.0));
                    if ui.add(open_button).clicked() {
                        match open_in_browser(&video.url) {
                            Ok(()) => {
                                state.status = "Opened video in browser.".into();
                            }
                            Err(err) => {
                                state.status = format!("Failed to open browser: {err}");
                            }
                        }
                    }
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let channel_label = channel_display_label(video);
                ui.label(format!("Channel: {}", channel_label));
                if state.is_channel_blocked(video) {
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
            ui.label(format!(
                "Duration: {}",
                format_duration(video.duration_secs)
            ));
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
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                ui.label(text.clone());
                            });
                    }
                });
            }
        });
}
