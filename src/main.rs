#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
//#![windows_subsystem = "windows"]

use smallstr::SmallString;
use serde::{Serialize, Deserialize};
use gilrs::{Gilrs, self};
use eframe::{epi, egui};

#[derive(Debug, Serialize, Deserialize)]
pub enum Action {
    None,
    /// Send this string
    Unicode(SmallString<[u8; 16]>),
    /// Press and release the key of the given name
    Key(SmallString<[u8; 16]>),
    /// Press all of the given keys in order, release in reverse order
    Combo(Vec<SmallString<[u8; 16]>>),
    /// Do all the actions given, in order
    Macro(Vec<Action>),
}

impl std::default::Default for Action {
    fn default() -> Self {
        Action::None
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ActionSet {
    lbutton: Action,
    rbutton: Action,
    n: Action,
    e: Action,
    s: Action,
    w: Action,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveState {
    actionsets: [ActionSet; 8], //clockwise from north
    circle_the_square: bool,
}

#[derive(Debug)]
pub struct App {
    state: SaveState,
    gil: Gilrs,
    active_gamepad: Option<gilrs::GamepadId>,
    lstick_coords: (f32, f32),
}

impl epi::App for App {
    fn name(&self) -> &str {
        "PadType"
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        storage: Option<&dyn epi::Storage>,
    ) {
        if let Some(storage) = storage {
            if let Some(loaded_state) = epi::get_value(storage, epi::APP_KEY) {
                self.state = loaded_state;
            }
        }
    }

    fn save(
        &mut self,
        storage: &mut dyn epi::Storage,
    ) {
        epi::set_value(storage, epi::APP_KEY, &self.state);
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &epi::Frame,
    ) {
        while let Some(gilrs::Event { id, event, time: _ }) = self.gil.next_event() {
            match event {
                gilrs::EventType::Connected => self.active_gamepad = Some(id),
                gilrs::EventType::Disconnected => {
                    if self.active_gamepad == Some(id) {
                        self.active_gamepad = None;
                    }
                },
                gilrs::EventType::AxisChanged(axis, val, _) => {
                    if Some(id) == self.active_gamepad {
                        match axis {
                            gilrs::Axis::LeftStickX => self.lstick_coords.0 = val,
                            gilrs::Axis::LeftStickY => self.lstick_coords.1 = val,
                            _ => (),
                        }
                    }
                },
                _ => (),
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Blarggg");
            egui::Grid::new("main_disp")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Active gamepad");
                    ui.label(format!("{:?}", self.active_gamepad));
                    ui.end_row();

                    ui.label("lstick x");
                    ui.label(format!("{}", self.lstick_coords.0));
                    ui.end_row();

                    ui.label("lstick y");
                    ui.label(format!("{}", self.lstick_coords.1));
                    ui.end_row();

                    ui.label("The time is now");
                    ui.label(format!("{:?}", std::time::SystemTime::now()));
                    ui.end_row();

                    if let Some(gamepad) = self.active_gamepad.map(|id| self.gil.gamepad(id)) {
                        ui.label("gp lsx");
                        ui.label(format!("{:?}",gamepad.axis_data(gilrs::Axis::LeftStickX)));
                        ui.end_row();
                        
                        ui.label("gp lsy");
                        ui.label(format!("{:?}",gamepad.axis_data(gilrs::Axis::LeftStickY)));
                        ui.end_row();
                        
                        ui.label("gp rsx");
                        ui.label(format!("{:?}",gamepad.axis_data(gilrs::Axis::RightStickX)));
                        ui.end_row();
                        
                        ui.label("gp rsy");
                        ui.label(format!("{:?}",gamepad.axis_data(gilrs::Axis::RightStickY)));
                        ui.end_row();
                        
                    }
                });
            egui::warn_if_debug_build(ui);
        });
    }
}

fn main() {
    let gil = Gilrs::new().unwrap();
    let mut active_gamepad = None;
    for (id, gp) in gil.gamepads() {
        println!("{:?}: {} {:?}", id, gp.name(), gp.power_info());
        active_gamepad = Some(id);
    }
    let app = App {
        state: Default::default(),
        gil,
        active_gamepad,
        lstick_coords: (0.0,0.0),
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
