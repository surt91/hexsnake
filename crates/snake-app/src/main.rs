#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("HexSnake"),
        ..Default::default()
    };
    eframe::run_native(
        "HexSnake",
        native_options,
        Box::new(|cc| Ok(Box::new(snake_app::App::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = eframe::web_sys::window()
            .expect("no window")
            .document()
            .expect("no document");

        let canvas = document
            .get_element_by_id("hexsnake_canvas")
            .expect("failed to find canvas element 'hexsnake_canvas'")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("'hexsnake_canvas' is not a canvas element");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(snake_app::App::new(cc)))),
            )
            .await;

        if let Err(e) = start_result {
            log::error!("failed to start eframe: {e:?}");
        }
    });
}
