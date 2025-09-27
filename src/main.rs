use YTSearch::ui;

fn main() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([1100.0, 600.0]);
    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "YTSearch",
        native_options,
        Box::new(|cc| Ok(Box::new(ui::AppState::new(cc)))),
    )
}
