use swayipc::{Connection, Fallible};

const TITLE: &'static str = "Sway Multi Monitor Setup";

fn main() -> Fallible<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(500.0, 500.0)),
        always_on_top: true,
        centered: true,
        decorated: false,
        resizable: true,
        transparent: false,
        ..Default::default()
    };

    let mut connection = Connection::new()?;

    println!("Getting new layout from sway:");
    let mut screen_grid = ScreenGrid::from_outputs(connection.get_outputs()?);
    screen_grid.print();
    screen_grid.recalculate_padding();
    println!("With gaps:");
    screen_grid.print();

    let myapp = MyApp {
        screen_grid,
        connection,
    };

    eframe::run_native(
        TITLE,
        options,
        Box::new(|_cc| Box::<MyApp>::new(myapp)),
    ).unwrap();

    Ok(())
}

mod screen_grid;
mod drag_and_drop;

use eframe::egui;
use eframe::egui::{Button, Id, Ui, Vec2};
use crate::drag_and_drop::{drag_source, drop_target};
use crate::screen_grid::ScreenGrid;

struct MyApp {
    screen_grid: ScreenGrid,
    connection: Connection,
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let id_source = "dragged_screen";
        let mut src_screen = None;
        let mut dst_screen = None;
        let can_accept_what_is_being_dragged = true;
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                let inner_size = egui::Resize::default()
                    .auto_sized()
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.vertical_centered(|ui| ui.heading(TITLE));
                            ui.vertical(|ui| {
                                for (row_idx, row) in self.screen_grid.inner.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        for (col_idx, screen) in row.iter().enumerate() {
                                            match screen {
                                                Some(screen) => {
                                                    let item_id = Id::new(id_source).with(&screen.name);
                                                    drag_source(ui, item_id, |ui| {
                                                        ui.add_sized(Vec2::new(108f32, 108f32), |ui: &mut Ui| {
                                                            ui.add_enabled(false, Button::new(format!("{}\n{}x{}", screen.name, screen.width, screen.height)))
                                                        });
                                                    });

                                                    if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                                                        src_screen = Some((row_idx, col_idx));
                                                    }
                                                }
                                                None => {
                                                    let response = drop_target(ui, can_accept_what_is_being_dragged, |ui| {
                                                        ui.add_sized(Vec2::new(100f32, 100f32), Button::new(""));
                                                    }).response;

                                                    let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());
                                                    if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
                                                        dst_screen = Some((row_idx, col_idx));
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }
                            });
                        })
                    }).response.rect.size();

                if let Some((src_row_idx, src_col_idx)) = src_screen {
                    if let Some((dst_row_idx, dst_col_idx)) = dst_screen {
                        if ui.input(|i| i.pointer.any_released()) {
                            println!("Moving from: {}x{} => {}x{}", src_row_idx, src_col_idx, dst_row_idx, dst_col_idx);
                            let value = std::mem::replace(&mut self.screen_grid.inner[src_row_idx][src_col_idx], None);
                            self.screen_grid.inner[dst_row_idx][dst_col_idx] = value;

                            println!("New screen layout after drop:");
                            self.screen_grid.print();
                            self.screen_grid.apply_changes(&mut self.connection);
                            println!("Getting new layout from sway:");
                            self.screen_grid = ScreenGrid::from_outputs(self.connection.get_outputs().unwrap());
                            self.screen_grid.print();
                            self.screen_grid.recalculate_padding();
                            println!("With gaps:");
                            self.screen_grid.print();
                        }
                    }
                }

                let outer_size = frame.info().window_info.size;
                if inner_size != outer_size {
                    println!("{}x{} => {}x{}", outer_size[0], outer_size[1], inner_size[0], inner_size[1]);
                    println!("Updating!");
                    // frame.set_window_size(inner_size);
                    // frame.set_window_pos(Vec2::new(1000f32, 1000f32).to_pos2());
                    println!("{}x{} => {}x{}", outer_size[0], outer_size[1], inner_size[0], inner_size[1]);
                }
            });
    }
}
