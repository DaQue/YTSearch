use crate::filters;
use crate::prefs::{self, Prefs, TimeWindowPreset};
use crate::search_runner::{self, RunMode, SearchOutcome};
use crate::yt::types::VideoDetails;
use egui::{self, Align, Color32, Context, CornerRadius, Frame, Layout, Margin, RichText, Stroke};
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};

const PRESET_COLORS: &[egui::Color32] = &[
    egui::Color32::from_rgb(0x4F, 0x90, 0xD9),
    egui::Color32::from_rgb(0xEE, 0x88, 0x3B),
    egui::Color32::from_rgb(0x5C, 0xB8, 0x5C),
    egui::Color32::from_rgb(0xD6, 0x4D, 0x57),
    egui::Color32::from_rgb(0x9A, 0x59, 0xD1),
];

const PANEL_FILL: Color32 = Color32::from_rgb(22, 22, 28);
const WINDOW_FILL: Color32 = Color32::from_rgb(15, 15, 20);
const CARD_BG: Color32 = Color32::from_rgb(32, 32, 40);
const CARD_BORDER: Color32 = Color32::from_rgb(55, 65, 81);
const STATUS_ACCENT: Color32 = Color32::from_rgb(99, 102, 241);
const ACCENT_SEARCH: Color32 = Color32::from_rgb(239, 68, 68); // red
const ACCENT_ANY: Color32 = Color32::from_rgb(249, 115, 22); // orange
const ACCENT_SINGLE: Color32 = Color32::from_rgb(250, 204, 21); // yellow
const ACCENT_SAVE: Color32 = Color32::from_rgb(34, 197, 94); // green
const ACCENT_OPEN: Color32 = Color32::from_rgb(59, 130, 246); // blue
const ACCENT_EXTRA: Color32 = Color32::from_rgb(168, 85, 247); // purple

enum SearchResult {
    Success(SearchOutcome),
    Error(String),
}

pub struct AppState {
    prefs: Prefs,
    status: String,
    run_any_mode: bool,
    results: Vec<VideoDetails>,
    runtime: Runtime,
    selected_search_id: Option<String>,
    pending_task: Option<JoinHandle<()>>,
    search_rx: Option<mpsc::Receiver<SearchResult>>,
    is_searching: bool,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_gfv_theme(&cc.egui_ctx);

        let mut prefs = prefs::load_or_default();
        prefs::add_missing_defaults(&mut prefs);
        prefs::normalize_block_list(&mut prefs.blocked_channels);
        let mut status = String::from("Ready.");

        if prefs.api_key.trim().is_empty() {
            let key_path = Path::new("YT_API_private");
            if let Ok(contents) = fs::read_to_string(key_path) {
                let trimmed = contents.trim();
                if !trimmed.is_empty() {
                    prefs.api_key = trimmed.to_owned();
                    status = "API key imported from YT_API_private.".into();
                }
            }
        }

        for search in &mut prefs.searches {
            if matches!(search.query.category_id, Some(28)) {
                search.query.category_id = None;
            }
        }
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to start tokio runtime");
        let selected_search_id = prefs.searches.first().map(|s| s.id.clone());
        Self {
            prefs,
            status,
            run_any_mode: true,
            results: Vec::new(),
            runtime,
            selected_search_id,
            pending_task: None,
            search_rx: None,
            is_searching: false,
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let mut search_requested = false;
        let incoming = if let Some(rx) = self.search_rx.as_mut() {
            match rx.try_recv() {
                Ok(msg) => Some(msg),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(SearchResult::Error("Search cancelled.".into()))
                }
            }
        } else {
            None
        };

        if let Some(message) = incoming {
            match message {
                SearchResult::Success(outcome) => {
                    let skipped_duplicates =
                        outcome.duplicates_within_presets + outcome.duplicates_across_presets;
                    let presets = outcome.presets_ran;
                    let pages = outcome.pages_fetched;
                    let raw = outcome.raw_items;
                    let unique = outcome.unique_ids;
                    let passed = outcome.passed_filters;
                    let blocked_keys = prefs::blocked_keys(&self.prefs.blocked_channels);
                    self.results = outcome
                        .videos
                        .into_iter()
                        .filter(|v| {
                            !filters::matches_channel(
                                &v.channel_handle,
                                &v.channel_title,
                                &blocked_keys,
                            )
                        })
                        .collect();
                    let kept = self.results.len();
                    self.status = format!(
                        "Ran {presets} preset(s) across {pages} page(s); raw {raw}, unique {unique}, passed {passed}, kept {kept} (skipped {skipped_duplicates} duplicates)."
                    );
                    self.is_searching = false;
                }
                SearchResult::Error(err) => {
                    self.status = format!("Search failed: {err}");
                    self.is_searching = false;
                }
            }
            self.search_rx = None;
            self.pending_task = None;
        }

        if let Some(selected) = self.selected_search_id.clone() {
            if !self.prefs.searches.iter().any(|s| s.id == selected) {
                self.selected_search_id = self.prefs.searches.first().map(|s| s.id.clone());
            }
        } else if let Some(first) = self.prefs.searches.first() {
            self.selected_search_id = Some(first.id.clone());
        }

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

        if search_requested {
            self.launch_search();
        }

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
                                        let open_button = egui::Button::new(
                                            RichText::new("Open").strong().color(Color32::WHITE),
                                        )
                                        .fill(ACCENT_OPEN)
                                        .min_size(egui::vec2(90.0, 26.0));
                                        if ui.add(open_button).clicked() {
                                            match open_in_browser(&video.url) {
                                                Ok(()) => {
                                                    self.status = "Opened video in browser.".into();
                                                }
                                                Err(err) => {
                                                    self.status =
                                                        format!("Failed to open browser: {err}");
                                                }
                                            }
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(format!("Channel: {}", video.channel_title));
                                    if self.is_channel_blocked(video) {
                                        ui.label(
                                            RichText::new("Blocked").color(ACCENT_EXTRA).strong(),
                                        );
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
                                        for (idx, preset_name) in
                                            video.source_presets.iter().enumerate()
                                        {
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
                        ui.add_space(6.0);
                    }
                });
                for (channel_id, channel_title) in block_requests {
                    self.block_channel(&channel_id, &channel_title);
                }
            }
        });
    }
}

impl AppState {
    fn launch_search(&mut self) {
        if let Some(handle) = self.pending_task.take() {
            handle.abort();
        }
        self.search_rx = None;
        self.results.clear();
        self.status = "Searching...".into();
        self.is_searching = true;

        let prefs_snapshot = self.prefs.clone();
        let mode = match self.determine_run_mode(&prefs_snapshot) {
            Ok(mode) => mode,
            Err(msg) => {
                self.status = msg;
                self.is_searching = false;
                return;
            }
        };

        let (tx, rx) = mpsc::channel();
        let task = self.runtime.spawn(async move {
            let result = search_runner::run_searches(prefs_snapshot, mode).await;
            let message = match result {
                Ok(outcome) => SearchResult::Success(outcome),
                Err(err) => SearchResult::Error(err.to_string()),
            };
            let _ = tx.send(message);
        });
        self.pending_task = Some(task);
        self.search_rx = Some(rx);
    }

    fn determine_run_mode(&self, prefs: &Prefs) -> Result<RunMode, String> {
        if self.run_any_mode {
            Ok(RunMode::Any)
        } else {
            let id = self
                .selected_search_id
                .clone()
                .or_else(|| prefs.searches.first().map(|s| s.id.clone()))
                .ok_or_else(|| "Add a preset before searching.".to_string())?;
            Ok(RunMode::Single(id))
        }
    }

    fn selected_search_name(&self) -> Option<String> {
        let target = self.selected_search_id.as_ref()?;
        self.prefs
            .searches
            .iter()
            .find(|s| &s.id == target)
            .map(|s| s.name.clone())
    }

    fn block_channel(&mut self, channel_id: &str, channel_title: &str) {
        let source = if !channel_id.trim().is_empty() {
            channel_id.trim()
        } else {
            channel_title.trim()
        };

        if source.is_empty() {
            self.status = "Channel identifier unavailable for blocking.".into();
            return;
        }

        let key = source.trim_start_matches('@').to_ascii_lowercase();
        if self
            .prefs
            .blocked_channels
            .iter()
            .any(|entry| prefs::parse_block_entry(entry).0 == key)
        {
            self.status = format!("Channel '{}' already blocked.", channel_title);
            return;
        }

        let label = if channel_title.trim().is_empty() {
            source.to_string()
        } else {
            channel_title.trim().to_string()
        };

        self.prefs
            .blocked_channels
            .push(format!("{}|{}", key, label));
        prefs::normalize_block_list(&mut self.prefs.blocked_channels);

        if let Err(err) = prefs::save(&self.prefs) {
            self.status = format!("Failed to save block list: {err}");
        } else {
            self.status = format!("Blocked channel: {}", channel_title);
        }

        let blocked_keys = prefs::blocked_keys(&self.prefs.blocked_channels);
        self.results.retain(|v| {
            !filters::matches_channel(&v.channel_handle, &v.channel_title, &blocked_keys)
        });
    }

    fn is_channel_blocked(&self, video: &VideoDetails) -> bool {
        let blocked_keys = prefs::blocked_keys(&self.prefs.blocked_channels);
        filters::matches_channel(&video.channel_handle, &video.channel_title, &blocked_keys)
    }

    fn unblock_channel(&mut self, channel_key: &str) {
        let target = channel_key
            .trim()
            .trim_start_matches('@')
            .to_ascii_lowercase();
        let original_len = self.prefs.blocked_channels.len();
        self.prefs
            .blocked_channels
            .retain(|entry| prefs::parse_block_entry(entry).0 != target);
        if self.prefs.blocked_channels.len() != original_len {
            prefs::normalize_block_list(&mut self.prefs.blocked_channels);
            if let Err(err) = prefs::save(&self.prefs) {
                self.status = format!("Failed to save block list: {err}");
            } else {
                self.status = format!("Unblocked channel: {}", channel_key);
            }
        }
    }
}

fn apply_gfv_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = WINDOW_FILL;
    visuals.panel_fill = PANEL_FILL;
    visuals.faint_bg_color = Color32::from_rgb(32, 32, 40);
    visuals.extreme_bg_color = Color32::from_rgb(42, 42, 50);
    visuals.selection.bg_fill = STATUS_ACCENT;
    visuals.hyperlink_color = STATUS_ACCENT;
    visuals.button_frame = true;
    visuals.window_stroke = Stroke::new(1.0, CARD_BORDER);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(12.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.window_margin = Margin::same(16);
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(22.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
    );
    style.visuals = visuals;
    ctx.set_style(style);
}

fn tinted_toggle_button(ui: &mut egui::Ui, active: bool, label: &str, color: Color32) -> bool {
    let fill = if active {
        color
    } else {
        color.linear_multiply(0.25)
    };
    let text_color = if active { contrast_text(color) } else { color };
    ui.add(
        egui::Button::new(RichText::new(label).strong().color(text_color))
            .min_size(egui::vec2(80.0, 28.0))
            .fill(fill),
    )
    .clicked()
}

fn contrast_text(color: Color32) -> Color32 {
    let brightness = color.r() as u32 * 299 + color.g() as u32 * 587 + color.b() as u32 * 114;
    if brightness > 128_000 {
        Color32::from_rgb(26, 32, 44)
    } else {
        Color32::WHITE
    }
}

fn time_window_label(preset: TimeWindowPreset) -> &'static str {
    match preset {
        TimeWindowPreset::Today => "Today",
        TimeWindowPreset::H48 => "48h",
        TimeWindowPreset::D7 => "7d",
        TimeWindowPreset::Custom => "Custom",
    }
}

fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        match try_launch_new_window(url) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                return open::that(url)
                    .map(|_| ())
                    .map_err(|e| format!("{err}; fallback failed: {e}"));
            }
            Err(_) => {}
        }
    }

    open::that(url).map(|_| ()).map_err(|err| err.to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_launch_new_window(url: &str) -> std::io::Result<()> {
    use std::io::ErrorKind;
    use std::process::Command;

    const CANDIDATES: [&str; 4] = [
        "google-chrome",
        "chromium",
        "brave-browser",
        "microsoft-edge",
    ];

    for cmd in CANDIDATES {
        match Command::new(cmd).arg("--new-window").arg(url).spawn() {
            Ok(_) => return Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        }
    }

    Err(std::io::Error::new(
        ErrorKind::NotFound,
        "no supported browser command found",
    ))
}
