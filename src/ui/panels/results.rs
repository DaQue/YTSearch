use egui::{
    Align2, Color32, Context, CornerRadius, FontId, Frame, Image, Margin, RichText, Sense, Stroke,
    StrokeKind,
};

use crate::ui::panels::helpers::channel_display_label;
use crate::ui::theme::{ACCENT_EXTRA, ACCENT_OPEN, CARD_BG, CARD_BORDER, PRESET_COLORS};
use crate::ui::utils::{format_duration, open_in_browser};
use crate::yt::types::VideoDetails;

use super::AppState;
use crate::ui::app_state::ResultSort;
use crate::ui::thumbnails::{MAX_THUMB_HEIGHT, MAX_THUMB_WIDTH, ThumbnailRef};

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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Downloaded: {}", state.results_all.len()));
            });
        });
        if state.is_searching {
            ui.label("Searching...");
        } else if state.results.is_empty() {
            ui.label("No results yet. Enter your API key and click Search.");
            ui.label(format!("Visible after filters: {}", state.results.len()));
        } else {
            let mut block_requests: Vec<(String, String)> = Vec::new();
            let results_snapshot = state.results.clone();
            let filtered_results: Vec<VideoDetails> = results_snapshot
                .into_iter()
                .filter(|video| state.duration_filter.allows(video.duration_secs))
                .collect();
            ui.label(format!("Visible after filters: {}", filtered_results.len()));
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
    let ctx = ui.ctx();
    let thumbnail = state.thumbnail_for_video(ctx, video);
    let thumb_loading = state.thumbnail_cache.is_loading(&video.id);
    let thumb_failed = state.thumbnail_cache.is_failed(&video.id);

    Frame::default()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, CARD_BORDER))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(MAX_THUMB_WIDTH);
                    render_thumbnail(ui, thumbnail.as_ref(), thumb_loading, thumb_failed, video);
                    ui.add_space(6.0);
                    render_open_button(state, ui, video);
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    render_title_row(ui, video);
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
                            if ui
                                .add(block_button)
                                .on_hover_text("Hide this channel in future results")
                                .clicked()
                            {
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
            });
        });
}

fn render_title_row(ui: &mut egui::Ui, video: &VideoDetails) {
    let title = RichText::new(&video.title)
        .heading()
        .color(Color32::from_rgb(229, 231, 235));
    let label = egui::Label::new(title).wrap();
    ui.add(label);
}

fn render_thumbnail(
    ui: &mut egui::Ui,
    thumbnail: Option<&ThumbnailRef>,
    is_loading: bool,
    is_failed: bool,
    video: &VideoDetails,
) {
    let desired = egui::vec2(MAX_THUMB_WIDTH, MAX_THUMB_HEIGHT);
    if let Some(thumb) = thumbnail {
        let texture_id = thumb.texture.id();
        let image =
            Image::new((texture_id, thumb.display_size)).corner_radius(CornerRadius::same(6));
        ui.add(image);
    } else {
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let rounding = CornerRadius::same(6);
        let bg = Color32::from_rgb(30, 34, 42);
        ui.painter().rect_filled(rect, rounding, bg);
        ui.painter().rect_stroke(
            rect,
            rounding,
            Stroke::new(1.0, CARD_BORDER),
            StrokeKind::Inside,
        );

        let message = if !video.thumbnail_url.as_deref().unwrap_or("").is_empty() {
            if is_failed {
                "Failed to load"
            } else if is_loading {
                "Loading…"
            } else {
                "Fetching thumbnail"
            }
        } else {
            "No thumbnail"
        };

        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            message,
            FontId::proportional(12.0),
            Color32::from_gray(180),
        );
    }
}

fn render_open_button(state: &mut AppState, ui: &mut egui::Ui, video: &VideoDetails) {
    let open_button = egui::Button::new(RichText::new("Open").strong().color(Color32::WHITE))
        .fill(ACCENT_OPEN)
        .min_size(egui::vec2(90.0, 26.0));
    let response = ui
        .add_sized(egui::vec2(MAX_THUMB_WIDTH, 30.0), open_button)
        .on_hover_text("Open video in your browser");
    if response.clicked() {
        match open_in_browser(&video.url) {
            Ok(()) => {
                state.status = "Opened video in browser.".into();
            }
            Err(err) => {
                state.status = format!("Failed to open browser: {err}");
            }
        }
    }
}
