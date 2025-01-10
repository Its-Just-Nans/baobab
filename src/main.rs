#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use baobab::SendType;
use boa_engine::{Context as BoaContext, Source};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    use std::sync::mpsc::channel;
    use std::thread;

    let (send_js, recv_js) = channel::<SendType>();
    let (send_res, recv_red) = channel::<SendType>();

    let receiver = thread::spawn(move || {
        let mut context = BoaContext::builder().build().unwrap();
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
                    send_res.send(SendType::Result(val)).unwrap();
                }
                _ => continue,
            }
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Ok(Box::new(baobab::TemplateApp::new(cc, send_js, recv_red)))),
    );
    receiver.join().expect("The receiver thread has panicked");

    Ok(())
}
