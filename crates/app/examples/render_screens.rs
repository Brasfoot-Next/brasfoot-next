//! Renders the app offscreen to PNG files, to compare with the game's own windows
//! (`bfmod.Vitrine`) and to check small windows. Usage:
//! BF_OPEN=<save> render_screens <png prefix> [<width>x<height>] [<tab>,<tab>,...]
//! (default 1360x768, the original's; tabs as `BF_TAB`, empty for the main screen).

use brasfoot_i18n::{DEFAULT_LOCALE, Translator};
use brasfoot_next::game_files::GameFiles;
use brasfoot_next::{App, theme};
use eframe::egui;
use egui_kittest::Harness;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prefix = args.get(1).cloned().unwrap_or_else(|| "screen".into());
    let size = args
        .get(2)
        .and_then(|s| s.split_once('x'))
        .and_then(|(w, h)| Some(egui::vec2(w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or(egui::vec2(1360.0, 768.0));
    let tabs: Vec<String> = args.get(3).map(|t| t.split(',').map(str::to_owned).collect()).unwrap_or_else(|| vec![String::new(), "tables".into(), "calendar".into()]);
    for tab in &tabs {
        let name = if tab.is_empty() { "principal" } else { tab.as_str() };
        // SAFETY: single-threaded setup before the app reads it
        unsafe { std::env::set_var("BF_TAB", tab) };
        let mut app = App::new(Translator::new(DEFAULT_LOCALE), GameFiles::locate());
        let mut ready = false;
        let mut harness = Harness::builder().with_size(size).wgpu().build_ui(|ui| {
            // the fonts and the loaders first: the test harness starts with minimal fonts
            if !ready {
                ui.ctx().set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
                egui_extras::install_image_loaders(ui.ctx());
                theme::apply(ui.ctx());
                ready = true;
                return;
            }
            app.show(ui)
        });
        // let the pictures load
        for _ in 0..30 {
            harness.step();
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        let image = harness.render().expect("render");
        let path = format!("{prefix}-{name}.png");
        image.save(&path).expect("save");
        println!("{path}");
    }
}
