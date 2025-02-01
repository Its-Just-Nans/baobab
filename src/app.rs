use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum SendType {
    Code(String),
    Result(String),
    Quit,
}

pub struct BaobabApp {
    // Example stuff:
    value: String,
    old_values: Vec<SendType>,
    send_js: Sender<SendType>,
    recv_res: Receiver<SendType>,
}

impl BaobabApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        send_js: Sender<SendType>,
        recv_res: Receiver<SendType>,
    ) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let saved = match cc.storage {
            Some(store) => eframe::get_value::<String>(store, eframe::APP_KEY),
            _ => None,
        };
        Self {
            old_values: Default::default(),
            value: saved.unwrap_or_default(),
            send_js,
            recv_res,
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

            egui::menu::bar(ui, |ui| {
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
                    self.send_js
                        .send(SendType::Code(self.value.clone()))
                        .unwrap();
                    self.value.clear();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if let Some(SendType::Code(s)) =
                        self.old_values.get(self.old_values.len().wrapping_sub(2))
                    {
                        self.value = s.to_string().replace(">> ", "");
                    }
                }
                self.recv_res.try_iter().for_each(|v| {
                    self.old_values.push(v);
                });
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
    }
}
