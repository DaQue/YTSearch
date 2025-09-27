mod app_state;
mod panels;
mod theme;
mod utils;

pub use app_state::AppState;
use app_state::SearchResult;

use crate::filters;
use crate::prefs;
use egui::Context;
use std::sync::mpsc::TryRecvError;

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Handle incoming search results
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

        // Validate selected search
        if let Some(selected) = self.selected_search_id.clone() {
            if !self.prefs.searches.iter().any(|s| s.id == selected) {
                self.selected_search_id = self.prefs.searches.first().map(|s| s.id.clone());
            }
        } else if let Some(first) = self.prefs.searches.first() {
            self.selected_search_id = Some(first.id.clone());
        }

        // Render panels
        let search_requested = self.render_top_panel(ctx);
        self.render_left_panel(ctx);
        self.render_central_panel(ctx);

        if search_requested {
            self.launch_search();
        }
    }
}
