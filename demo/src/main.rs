//! An interactive demo of egui_gridflow: real egui-pop widgets laid out in
//! a box you resize, with the engine's choices drawn over them.
//!
//! On the web: `trunk serve` in this directory. On the desktop:
//! `cargo run -p egui_gridflow_demo`.

mod app;
mod catalog;
mod render;

use egui_pop::eframe;

/// The eframe application around the demo.
struct DemoApp(app::Demo);

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui_pop::egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
}

fn create(_cc: &eframe::CreationContext<'_>) -> DemoApp {
    DemoApp(app::Demo::default())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    eframe::run_native(
        "egui_gridflow demo",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(create(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let _ = eframe::WebLogger::init(log::LevelFilter::Info);
    // A panic aborts the module: say so in the page, not with a black
    // canvas.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        failed(&format!("it panicked: {info}"));
        previous(info);
    }));

    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = eframe::web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(canvas) = document
            .get_element_by_id("the_canvas_id")
            .and_then(|e| e.dyn_into::<eframe::web_sys::HtmlCanvasElement>().ok())
        else {
            failed("the page has no canvas `the_canvas_id`");
            return;
        };
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions {
                    should_prevent_default: Box::new(egui_pop::core::pointer::web_prevent_default),
                    ..eframe::WebOptions::default()
                },
                Box::new(|cc| Ok(Box::new(create(cc)))),
            )
            .await;
        if let Err(e) = started {
            failed(&format!("it did not start: {e:?}"));
        }
    });
}

/// Logs why the demo stopped, and shows it in the page in place of the
/// canvas.
#[cfg(target_arch = "wasm32")]
fn failed(why: &str) {
    log::error!("egui_gridflow demo: {why}");
    let Some(document) = eframe::web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(canvas) = document.get_element_by_id("the_canvas_id") {
        let _ = canvas.set_attribute("hidden", "");
    }
    if let Some(message) = document.get_element_by_id("failed") {
        message.set_text_content(Some(&format!(
            "The demo could not run in this browser: {why}. The browser's console says more."
        )));
        let _ = message.remove_attribute("hidden");
    }
}
