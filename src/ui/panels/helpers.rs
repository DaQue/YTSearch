use egui::{Frame, Key, Margin, RichText, Stroke, TextEdit};

use crate::ui::preset_editor::PresetEditorState;
use crate::ui::theme::PRESET_COLORS;
use crate::yt::types::VideoDetails;

pub(super) fn render_token_editor(
    ui: &mut egui::Ui,
    label: &str,
    tokens: &mut Vec<String>,
    new_token: &mut String,
    hint: &str,
) {
    ui.label(label);

    let mut removals: Vec<usize> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for (idx, token) in tokens.iter().enumerate() {
            let color = PRESET_COLORS[idx % PRESET_COLORS.len()];
            let fill = color.linear_multiply(0.15);
            let stroke = Stroke::new(1.0, color);
            Frame::default()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(token).color(color));
                        ui.add_space(6.0);
                        if ui.small_button("×").clicked() {
                            removals.push(idx);
                        }
                    });
                });
        }
    });

    if !removals.is_empty() {
        removals.sort_unstable();
        removals.drain(..).rev().for_each(|idx| {
            if idx < tokens.len() {
                tokens.remove(idx);
            }
        });
        PresetEditorState::normalize_terms(tokens);
    }

    ui.horizontal(|ui| {
        let response = ui.add(TextEdit::singleline(new_token).hint_text(hint));
        let mut commit = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if ui.button("Add").clicked() {
            commit = true;
        }

        if commit {
            let value = new_token.trim();
            if !value.is_empty()
                && !tokens
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(value))
            {
                tokens.push(value.to_string());
                PresetEditorState::normalize_terms(tokens);
            }
            new_token.clear();
        }
    });
}

pub(super) fn channel_display_label(video: &VideoDetails) -> String {
    let preferred_name = video
        .channel_display_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            let trimmed = video.channel_title.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

    let handle = video
        .channel_custom_url
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    match (preferred_name, handle) {
        (Some(name), Some(handle)) => {
            if handle.eq_ignore_ascii_case(&name) {
                name
            } else {
                format!("{} ({})", name, handle)
            }
        }
        (Some(name), None) => name,
        (None, Some(handle)) => handle,
        (None, None) => video.channel_handle.clone(),
    }
}
