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
#![warn(clippy::multiple_crate_versions)]

mod app;
pub use app::BaobabApp;
pub use app::SendType;

/// Run the Baobab JS engine thread
pub fn run_baobab_thread(
    recv_js: std::sync::mpsc::Receiver<SendType>,
    send_res: std::sync::mpsc::Sender<SendType>,
) -> std::thread::JoinHandle<Result<(), std::io::Error>> {
    use boa_engine::{Context, Source};
    use std::thread;
    thread::spawn(move || {
        let mut context = Context::builder()
            .build()
            .map_err(|e| std::io::Error::other(format!("Failed to create Boa Context: {}", e)))?;
        while let Ok(sent) = recv_js.recv() {
            match sent {
                SendType::Quit => break,
                SendType::Code(codeline) => {
                    let val = match context.eval(Source::from_bytes(&codeline)) {
                        Ok(res) => {
                            format!("{}", res.display())
                        }
                        Err(e) => e.to_string(),
                    };
                    send_res.send(SendType::Result(val)).map_err(|e| {
                        std::io::Error::other(format!(
                            "Failed to send result back to main thread: {}",
                            e
                        ))
                    })?;
                }
                _ => continue,
            }
        }
        Ok(())
    })
}

/// Run the Baobab App
/// # Errors
/// Returns `std::io::Error` if the JS engine thread panics
pub fn run_baobab() -> Result<(), std::io::Error> {
    use std::io::Error;
    use std::sync::mpsc::channel;

    let (send_js, recv_js) = channel::<SendType>();
    let (send_res, recv_red) = channel::<SendType>();

    let receiver = run_baobab_thread(recv_js, send_res);

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "baobab",
        native_options,
        Box::new(|cc| Ok(Box::new(BaobabApp::new(cc, send_js, recv_red)))),
    );
    receiver
        .join()
        .map_err(|_e| Error::other("The receiver thread has panicked"))?
}
