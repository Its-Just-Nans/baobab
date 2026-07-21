//! Baobab App Ui

use eframe::egui;

/// Types of messages sent between UI and JS engine
#[derive(Debug)]
pub enum SendType {
    /// Send Code as string to evaluate
    Code(String),
    /// Result from evaluation
    Result(String),
    /// Quit signal
    Quit,
}

/// The Baobab App
pub struct BaobabApp {
    /// Current value
    value: String,
    /// Previous values
    old_values: Vec<SendType>,
    /// Sender to JS engine
    /// Receiver from JS engine
    #[cfg(target_arch = "wasm32")]
    channels: (Option<SendType>, Option<SendType>),
    /// Sender to JS engine
    /// Receiver from JS engine
    #[cfg(not(target_arch = "wasm32"))]
    channels: (
        std::sync::mpsc::Sender<SendType>,
        std::sync::mpsc::Receiver<SendType>,
    ),
    /// JS context
    #[cfg(target_arch = "wasm32")]
    context: boa_engine::Context,
}

impl BaobabApp {
    /// Create a new Baobab App
    /// # Errors
    /// Can fail on wasm
    pub fn try_new(
        cc: &eframe::CreationContext<'_>,
        #[cfg(not(target_arch = "wasm32"))] channels: (
            std::sync::mpsc::Sender<SendType>,
            std::sync::mpsc::Receiver<SendType>,
        ),
        #[cfg(target_arch = "wasm32")] channels: (Option<SendType>, Option<SendType>),
    ) -> Result<Self, String> {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let saved = match cc.storage {
            Some(store) => eframe::get_value::<String>(store, eframe::APP_KEY),
            _ => None,
        };
        #[cfg(target_arch = "wasm32")]
        let context = boa_engine::Context::builder()
            .build()
            .map_err(|e| format!("Failed to create Boa Context: {}", e))?;
        Ok(Self {
            old_values: Default::default(),
            value: saved.unwrap_or_default(),
            #[cfg(target_arch = "wasm32")]
            context,
            channels,
        })
    }

    /// Send a command
    pub(crate) fn send_command(&mut self, value: SendType) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = self.channels.0.send(value) {
                return Err(format!("Failed to send code to JS engine: {}", e));
            }
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.channels.0 = Some(value);
            Ok(())
        }
    }

    /// Receive a command
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn receive_command(&mut self) -> Option<SendType> {
        self.channels.0.take()
    }

    /// Send a result
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn send_result(&mut self, value: SendType) -> Result<(), String> {
        self.channels.1 = Some(value);
        Ok(())
    }

    /// receive a result
    pub(crate) fn receive_result(&mut self) -> Option<SendType> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.channels.1.try_recv().ok()
        }
        #[cfg(target_arch = "wasm32")]
        self.channels.1.take()
    }

    /// Run the baobab background thread
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn run_baobab_thread(
        recv_js: std::sync::mpsc::Receiver<SendType>,
        send_res: std::sync::mpsc::Sender<SendType>,
    ) -> std::thread::JoinHandle<Result<(), std::io::Error>> {
        use boa_engine::{Context, Source};
        /// Run the Baobab JS engine thread
        use std::thread;
        thread::spawn(move || {
            let mut context = Context::builder().build().map_err(|e| {
                std::io::Error::other(format!("Failed to create Boa Context: {}", e))
            })?;
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

    /// run the baobab in wasm
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn run_baobab(&mut self, ctx: egui::Context) {
        let Some(value) = self.receive_command() else {
            return;
        };
        let context = &mut self.context;
        match value {
            SendType::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            SendType::Code(codeline) => {
                let val = match context.eval(boa_engine::Source::from_bytes(&codeline)) {
                    Ok(res) => {
                        format!("{}", res.display())
                    }
                    Err(e) => e.to_string(),
                };
                if let Err(_e) = self.send_result(SendType::Result(val)) {
                    // TODO handle
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for BaobabApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.value);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(">>");
                let wid = ui.text_edit_singleline(&mut self.value);
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.old_values.push(SendType::Code(format!(
                        "{} {}",
                        ">>",
                        self.value.clone()
                    )));
                    if let Err(_e) = self.send_command(SendType::Code(self.value.clone())) {
                        //TODO handle error
                    }
                    self.value.clear();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if let Some(SendType::Code(s)) =
                        self.old_values.get(self.old_values.len().wrapping_sub(2))
                    {
                        self.value = s.to_string().replace(">> ", "");
                    }
                    let text_edit_id = wid.id;
                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
                        let ccursor = egui::text::CCursor::new(self.value.chars().count());
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        state.store(ui.ctx(), text_edit_id);
                        ui.ctx().memory_mut(|mem| mem.request_focus(text_edit_id));
                    }
                }
                if let Some(result) = self.receive_result() {
                    self.old_values.push(result);
                }
                wid.request_focus();
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let scroll_area = egui::ScrollArea::vertical()
                .max_height(rect.height())
                .stick_to_bottom(true);
            scroll_area.show(ui, |ui| {
                self.old_values.iter().for_each(|v| match v {
                    SendType::Code(c) => {
                        ui.label(c);
                    }
                    SendType::Result(r) => {
                        ui.label(r);
                        ui.separator();
                    }
                    _ => {}
                });
            });
        });
        #[cfg(target_arch = "wasm32")]
        self.run_baobab(ctx.clone());
    }
}
