//! Application entry point (desktop & web).

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use brasfoot_i18n::{DEFAULT_LOCALE, Translator};
    use brasfoot_next::game_files::GameFiles;
    use brasfoot_next::patches::Library;
    use brasfoot_next::{App, theme};
    use eframe::egui;

    let translator = Translator::new(DEFAULT_LOCALE);
    let title = translator.text("app-title", None);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    let mut options = options;
    // development: BF_FPS measures without the screen's refresh as the ceiling
    if std::env::var_os("BF_FPS").is_some() {
        options.wgpu_options.surface.present_mode = eframe::wgpu::PresentMode::AutoNoVsync;
    }
    eframe::run_native(
        &title,
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx);
            if let Some(zoom) = std::env::var("BF_ZOOM").ok().and_then(|z| z.parse::<f32>().ok()) {
                cc.egui_ctx.set_zoom_factor(zoom);
            }
            Ok(Box::new(App::new(translator, GameFiles::locate().with_patches(Library::open()))))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use brasfoot_i18n::{DEFAULT_LOCALE, Translator};
    use brasfoot_next::game_files::GameFiles;
    use brasfoot_next::patches::Library;
    use brasfoot_next::{App, theme};
    use wasm_bindgen::JsCast;

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("no global window exists")
            .document()
            .expect("should have a document on window");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("should have canvas with id the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    theme::apply(&cc.egui_ctx);
                    let translator = Translator::new(DEFAULT_LOCALE);
                    Ok(Box::new(App::new(translator, GameFiles::empty().with_patches(Library::open()))))
                }),
            )
            .await;

        if let Err(e) = start_result {
            log::error!("Failed to start eframe: {e:?}");
        }
    });
}
