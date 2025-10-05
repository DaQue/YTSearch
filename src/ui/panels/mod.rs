use egui::Context;

use super::app_state::AppState;

mod editor;
mod helpers;
mod import_export;
mod left;
mod results;
mod top;

impl AppState {
    pub fn render_top_panel(&mut self, ctx: &Context) -> bool {
        top::render(self, ctx)
    }

    pub fn render_left_panel(&mut self, ctx: &Context) {
        left::render(self, ctx);
    }

    pub fn render_central_panel(&mut self, ctx: &Context) {
        results::render(self, ctx);
    }

    pub fn render_editor_window(&mut self, ctx: &Context) {
        editor::render(self, ctx);
    }

    pub fn render_import_export_windows(&mut self, ctx: &Context) {
        import_export::render(self, ctx);
    }
}
