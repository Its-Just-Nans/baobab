//! Baobab App Ui

use bladvak::BladvakApp;
#[cfg(target_arch = "wasm32")]
use bladvak::ErrorManager;
use bladvak::eframe;
use bladvak::eframe::egui;

/// Types of messages sent between UI and JS engine
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum SendType {
    /// Send Code as string to evaluate
    Code(String),
    /// Result from evaluation
    Result(String),
    /// Quit signal
    Quit,
}

/// The Baobab App
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct BaobabApp {
    /// Current value
    value: String,
    /// Previous values
    old_values: Vec<SendType>,
    /// Sender to JS engine
    /// Receiver from JS engine
    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    channels: (Option<SendType>, Option<SendType>),
    /// Sender to JS engine
    /// Receiver from JS engine
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    channels: Option<(
        std::sync::mpsc::Sender<SendType>,
        std::sync::mpsc::Receiver<SendType>,
    )>,
    /// JS context
    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    context: boa_engine::Context,
}

impl BaobabApp {
    /// Send a command
    pub(crate) fn send_command(&mut self, value: SendType) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(channels) = &self.channels else {
                return Err("No channels".to_string());
            };
            if let Err(e) = channels.0.send(value) {
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
            let Some(channels) = &self.channels else {
                return None;
            };
            channels.1.try_recv().ok()
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
    pub(crate) fn run_baobab(&mut self, ctx: &egui::Context, error_manager: &mut ErrorManager) {
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
                if let Err(err) = self.send_result(SendType::Result(val)) {
                    error_manager.add_error(err);
                }
            }
            _ => {}
        }
    }
}

impl BladvakApp<'_> for BaobabApp {
    fn try_new_with_args(
        saved_state: Self,
        _cc: &eframe::CreationContext<'_>,
        _args: &[String],
        _error_manager: &mut bladvak::ErrorManager,
    ) -> Result<Self, bladvak::AppError> {
        let Self {
            old_values, value, ..
        } = saved_state;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::mpsc::channel;
            let (send_js, recv_js) = channel::<SendType>();
            let (send_res, recv_red) = channel::<SendType>();

            let handle = BaobabApp::run_baobab_thread(recv_js, send_res);
            drop(handle);
            let channels = Some((send_js, recv_red));
            Ok(Self {
                old_values,
                value,
                channels,
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let context = boa_engine::Context::builder()
                .build()
                .map_err(|e| format!("Failed to create Boa Context: {}", e))?;
            let channels = (None, None);
            Ok(Self {
                old_values,
                value,
                #[cfg(target_arch = "wasm32")]
                context,
                channels,
            })
        }
    }

    fn top_panel(&mut self, _ui: &mut egui::Ui, _error_manager: &mut bladvak::ErrorManager) {}

    fn handle_file(&mut self, _bytes: bladvak::File) -> Result<(), bladvak::AppError> {
        Ok(())
    }

    fn menu_file(&mut self, _ui: &mut egui::Ui, _error_manager: &mut bladvak::ErrorManager) {}
    fn name() -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn repo_url() -> String {
        "https://github.com/Its-Just-Nans/baobab".to_string()
    }

    fn is_open_button(&self) -> bool {
        false
    }

    fn central_panel(
        &mut self,
        ui: &mut bladvak::eframe::egui::Ui,
        error_manager: &mut bladvak::ErrorManager,
    ) {
        egui::Panel::bottom("bottom_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(">>");
                let wid = ui.text_edit_singleline(&mut self.value);
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.old_values.push(SendType::Code(format!(
                        "{} {}",
                        ">>",
                        self.value.clone()
                    )));
                    if let Err(err) = self.send_command(SendType::Code(self.value.clone())) {
                        error_manager.add_error(err);
                    }
                    wid.request_focus();
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
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
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
        self.run_baobab(ui.ctx(), error_manager);
    }
}
