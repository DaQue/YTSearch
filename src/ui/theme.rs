use egui::{Color32, FontFamily, FontId, Margin, RichText, Stroke, TextStyle};

pub const PRESET_COLORS: &[egui::Color32] = &[
    egui::Color32::from_rgb(0x4F, 0x90, 0xD9),
    egui::Color32::from_rgb(0xEE, 0x88, 0x3B),
    egui::Color32::from_rgb(0x5C, 0xB8, 0x5C),
    egui::Color32::from_rgb(0xD6, 0x4D, 0x57),
    egui::Color32::from_rgb(0x9A, 0x59, 0xD1),
];

pub const PANEL_FILL: Color32 = Color32::from_rgb(22, 22, 28);
pub const WINDOW_FILL: Color32 = Color32::from_rgb(15, 15, 20);
pub const CARD_BG: Color32 = Color32::from_rgb(32, 32, 40);
pub const CARD_BORDER: Color32 = Color32::from_rgb(55, 65, 81);
pub const STATUS_ACCENT: Color32 = Color32::from_rgb(99, 102, 241);
pub const ACCENT_SEARCH: Color32 = Color32::from_rgb(239, 68, 68); // red
pub const ACCENT_ANY: Color32 = Color32::from_rgb(249, 115, 22); // orange
pub const ACCENT_SINGLE: Color32 = Color32::from_rgb(250, 204, 21); // yellow
pub const ACCENT_SAVE: Color32 = Color32::from_rgb(34, 197, 94); // green
pub const ACCENT_OPEN: Color32 = Color32::from_rgb(59, 130, 246); // blue
pub const ACCENT_EXTRA: Color32 = Color32::from_rgb(168, 85, 247); // purple

pub fn apply_gfv_theme(ctx: &egui::Context) {
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
        TextStyle::Heading,
        FontId::new(22.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(15.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );
    style.visuals = visuals;
    ctx.set_style(style);
}

pub fn tinted_toggle_button(ui: &mut egui::Ui, active: bool, label: &str, color: Color32) -> bool {
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

pub fn contrast_text(color: Color32) -> Color32 {
    let brightness = color.r() as u32 * 299 + color.g() as u32 * 587 + color.b() as u32 * 114;
    if brightness > 128_000 {
        Color32::from_rgb(26, 32, 44)
    } else {
        Color32::WHITE
    }
}
