use std::{
    fs::File,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{channel, Sender},
        Arc,
    },
    thread,
};

use eframe::{
    egui::{self, Slider, TextEdit, TextureOptions},
    epaint::{mutex::Mutex, ColorImage},
};
use emulator_core::{Cartridge, Cpu, Key, Lcd, Rgb555, CYCLES_PER_SECOND};
use rfd::FileDialog;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rust GBA Emulator",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    );
}

#[derive(Debug)]
enum EmulatorCommand {
    Run,
    Pause,
    Step(u64),
    LoadRom(PathBuf),
    KeyPressed(Key),
    KeyReleased(Key),
}

#[derive(Debug)]
enum EmulatorState {
    Running,
    Paused,
}

struct MyEguiApp {
    display_buffer: Arc<Mutex<[[Rgb555; Lcd::LCD_WIDTH]; Lcd::LCD_HEIGHT]>>,
    emulator_command_sender: Sender<EmulatorCommand>,
    step_count: u64,
    cycles_executed: Arc<AtomicU64>,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        let display_buffer = Arc::new(Mutex::new(
            [[Rgb555::default(); Lcd::LCD_WIDTH]; Lcd::LCD_HEIGHT],
        ));
        let cycles_executed = Arc::new(AtomicU64::new(0));

        let (emulator_command_sender, emulator_command_receiver) = channel();

        {
            let display_buffer = Arc::clone(&display_buffer);
            let cycles_executed = Arc::clone(&cycles_executed);
            thread::spawn(move || {
                let cartridge = Cartridge::new(
                    include_bytes!("../../emulator-core/tests/suite.gba").as_slice(),
                    None,
                )
                .unwrap();
                let mut cpu = Cpu::new(cartridge);
                let mut state = EmulatorState::Running;

                loop {
                    for command in emulator_command_receiver.try_iter() {
                        match command {
                            EmulatorCommand::Pause => state = EmulatorState::Paused,
                            EmulatorCommand::Run => state = EmulatorState::Running,
                            EmulatorCommand::Step(count) => {
                                let cycle_start = cpu.cycle_count();
                                while (cpu.cycle_count() - cycle_start) < count {
                                    cpu.fetch_decode_execute();
                                }

                                state = EmulatorState::Paused
                            }
                            EmulatorCommand::LoadRom(path) => {
                                let file = File::open(path).unwrap();
                                let cartridge = Cartridge::new(file, None).unwrap();
                                cpu = Cpu::new(cartridge);
                            }
                            EmulatorCommand::KeyPressed(key) => {
                                cpu.bus.keypad.set_pressed(key, true)
                            }
                            EmulatorCommand::KeyReleased(key) => {
                                cpu.bus.keypad.set_pressed(key, false)
                            }
                        }
                    }

                    match state {
                        EmulatorState::Running => {
                            let cycle_start = cpu.cycle_count();
                            while (cpu.cycle_count() - cycle_start) < (CYCLES_PER_SECOND / 60) {
                                cpu.fetch_decode_execute();
                            }
                        }
                        EmulatorState::Paused => {}
                    }

                    {
                        display_buffer
                            .lock()
                            .copy_from_slice(cpu.bus.lcd.get_buffer());
                        cycles_executed.store(cpu.cycle_count(), Ordering::Relaxed);
                    }
                }
            });
        }

        Self {
            display_buffer,
            emulator_command_sender,
            step_count: 1,
            cycles_executed,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::Window::new("Controls").show(ctx, |ui| {
            if ui.button("Play").clicked() {
                self.emulator_command_sender
                    .send(EmulatorCommand::Run)
                    .unwrap();
            }

            if ui.button("Pause").clicked() {
                self.emulator_command_sender
                    .send(EmulatorCommand::Pause)
                    .unwrap();
            }

            ui.horizontal(|ui| {
                if ui.button("Step").clicked() {
                    self.emulator_command_sender
                        .send(EmulatorCommand::Step(self.step_count))
                        .unwrap();
                }
                ui.add(
                    Slider::new(&mut self.step_count, 1..=10_000_000)
                        .logarithmic(true)
                        .suffix(" step(s)"),
                );
            });

            if ui.button("Choose File").clicked() {
                let sender = self.emulator_command_sender.clone();
                thread::spawn(move || {
                    let file = FileDialog::new()
                        .add_filter("GBA ROM", &["gba"])
                        .pick_file()
                        .unwrap();

                    sender.send(EmulatorCommand::LoadRom(file))
                });
            }
        });

        egui::Window::new("Stats").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Steps:");
                ui.add(
                    TextEdit::singleline(&mut format!(
                        "{}",
                        self.cycles_executed.load(Ordering::Relaxed)
                    ))
                    .interactive(false),
                );
            });
        });

        egui::Window::new("Emulator Window")
            .collapsible(false)
            .default_height(Lcd::LCD_HEIGHT as f32 * 4.0)
            .default_width(Lcd::LCD_WIDTH as f32 * 4.0)
            .show(ctx, |ui| {
                let rgb_data = self
                    .display_buffer
                    .lock()
                    .iter()
                    .flat_map(|row| {
                        row.iter().flat_map(|pixel| {
                            let red = (pixel.red << 3) | (pixel.red >> 2);
                            let green = (pixel.green << 3) | (pixel.green >> 2);
                            let blue = (pixel.blue << 3) | (pixel.blue >> 2);
                            [red, green, blue]
                        })
                    })
                    .collect::<Vec<_>>();

                let image = ColorImage::from_rgb([Lcd::LCD_WIDTH, Lcd::LCD_HEIGHT], &rgb_data);
                let texture = ui
                    .ctx()
                    .load_texture("gba-texture", image, TextureOptions::NEAREST);

                ui.image(texture.id(), ui.available_size());

                const KEYS_TO_CHECK: &[Key] = &[
                    Key::A,
                    Key::B,
                    Key::Down,
                    Key::Left,
                    Key::Right,
                    Key::Up,
                    Key::Select,
                    Key::Start,
                    Key::L,
                    Key::R,
                ];

                for to_check in KEYS_TO_CHECK.iter().copied() {
                    let egui_key = match to_check {
                        Key::A => egui::Key::X,
                        Key::B => egui::Key::Z,
                        Key::Up => egui::Key::ArrowUp,
                        Key::Down => egui::Key::ArrowDown,
                        Key::Left => egui::Key::ArrowLeft,
                        Key::Right => egui::Key::ArrowRight,
                        Key::Start => egui::Key::Enter,
                        Key::Select => egui::Key::Space,
                        Key::L => egui::Key::Q,
                        Key::R => egui::Key::P,
                    };

                    if ctx.input().key_pressed(egui_key) {
                        self.emulator_command_sender
                            .send(EmulatorCommand::KeyPressed(to_check))
                            .unwrap();
                    } else if ctx.input().key_released(egui_key) {
                        self.emulator_command_sender
                            .send(EmulatorCommand::KeyReleased(to_check))
                            .unwrap();
                    };
                }
            });
    }
}
