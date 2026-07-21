//! Baobab is [`boa_cli`] in [`egui`]
//!
//! The basic usage is for a quick and easy js playground (for example, bind a keyboard shortcut to `baobab`).
//!
//! ## Usage
//!
//! ```sh
//! cargo install baobab
//! baobab
//! ```

#![warn(clippy::all, rust_2018_idioms)]
#![deny(
    missing_docs,
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::expect_used
)]
#![allow(clippy::multiple_crate_versions)]

mod app;
pub use app::BaobabApp;
pub use app::SendType;

/// Run the Baobab App
/// # Errors
/// Returns `std::io::Error` if the JS engine thread panics
#[cfg(not(target_arch = "wasm32"))]
pub fn run_baobab() -> Result<(), std::io::Error> {
    use std::sync::mpsc::channel;

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let (send_js, recv_js) = channel::<SendType>();
    let (send_res, recv_red) = channel::<SendType>();

    let receiver = BaobabApp::run_baobab_thread(recv_js, send_res);

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "baobab",
        native_options,
        Box::new(|cc| Ok(Box::new(BaobabApp::try_new(cc, (send_js, recv_red))?))),
    );
    receiver
        .join()
        .map_err(|_e| std::io::Error::other("The receiver thread has panicked"))?
}

/// Wasm run
#[cfg(target_arch = "wasm32")]
pub fn run_baobab() -> Result<(), std::io::Error> {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");
        log::error!("sdqfsd");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(BaobabApp::try_new(cc, (None, None))?))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start app: {e:?}");
                }
            }
        }
    });
    Ok(())
}
