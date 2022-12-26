use std::{
    array,
    fmt::Debug,
    fs::File,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    thread,
};

use eframe::{
    egui::{self, CollapsingHeader, ScrollArea, Slider, TextEdit, TextStyle, TextureOptions, Ui},
    epaint::ColorImage,
};
use emulator_core::{
    Bus, Cartridge, Cpu, CpuMode, Instruction, InstructionSet, Key, Lcd, Register, Rgb555,
    CYCLES_PER_SECOND,
};
use rfd::FileDialog;

fn main() {
    env_logger::init();

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
    CreateNewSaveState,
    UpdateSaveState(usize),
    LoadSaveState(usize),
}

#[derive(Debug)]
enum EmulatorState {
    Running,
    Paused,
}

struct DisassemblyInfo {
    pc: u32,
}

struct RegisterValue {
    register: Register,
    value: u32,
}

struct CpuInfo {
    sign_flag: bool,
    zero_flag: bool,
    carry_flag: bool,
    overflow_flag: bool,
    irq_disable: bool,
    fiq_disable: bool,
    instruction_mode: InstructionSet,
    cpu_mode: CpuMode,
    irq_buffer: [u16; Bus::IRQ_SYNC_BUFFER],
    open_bus_data: u32,
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self {
            sign_flag: false,
            zero_flag: false,
            carry_flag: false,
            overflow_flag: false,
            irq_disable: false,
            fiq_disable: false,
            instruction_mode: InstructionSet::Arm,
            cpu_mode: CpuMode::System,
            irq_buffer: [0; Bus::IRQ_SYNC_BUFFER],
            open_bus_data: Default::default(),
        }
    }
}

#[derive(Clone, Default)]
struct BreakpointInfo {
    address: u32,
    active: bool,
}

#[derive(Clone, Default)]
struct TimerInfo {
    reload: u16,
    counter: u16,
}

struct MyEguiApp {
    display_buffer: Arc<Mutex<[[Rgb555; Lcd::LCD_WIDTH]; Lcd::LCD_HEIGHT]>>,
    disassembly_info: Arc<Mutex<DisassemblyInfo>>,
    registers_info: Arc<Mutex<Box<[RegisterValue]>>>,
    cpu_info: Arc<Mutex<CpuInfo>>,
    timer_info: Arc<Mutex<Box<[TimerInfo]>>>,
    breakpoints: Arc<Mutex<Vec<BreakpointInfo>>>,
    emulator_command_sender: Sender<EmulatorCommand>,
    step_count: u64,
    cycles_executed: Arc<AtomicU64>,
    num_save_states: Arc<AtomicUsize>,
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
        let disassembly_info = Arc::new(Mutex::new(DisassemblyInfo { pc: 0x00000000 }));
        let registers_info = Arc::new(Mutex::new(Box::new([]) as Box<[_]>));
        let cpu_info = Arc::new(Mutex::new(CpuInfo::default()));
        let breakpoints = Arc::new(Mutex::new(Vec::<BreakpointInfo>::new()));
        let timer_info = Arc::new(Mutex::new(Box::new([]) as Box<[_]>));

        let cycles_executed = Arc::new(AtomicU64::new(0));
        let num_save_states = Arc::new(AtomicUsize::new(0));

        let (emulator_command_sender, emulator_command_receiver) = channel();

        {
            let display_buffer = Arc::clone(&display_buffer);
            let cycles_executed = Arc::clone(&cycles_executed);
            let disassembly_info = Arc::clone(&disassembly_info);
            let registers_info = Arc::clone(&registers_info);
            let cpu_info = Arc::clone(&cpu_info);
            let breakpoints = Arc::clone(&breakpoints);
            let timer_info = Arc::clone(&timer_info);
            let num_save_states = Arc::clone(&num_save_states);

            thread::spawn(move || {
                let cartridge = Cartridge::new(
                    include_bytes!("../../emulator-core/tests/suite.gba").as_slice(),
                    None,
                )
                .unwrap();
                let mut cpu = Cpu::new(cartridge);
                let mut state = EmulatorState::Paused;

                let mut save_states = Vec::new();

                loop {
                    for command in emulator_command_receiver.try_iter() {
                        match command {
                            EmulatorCommand::Pause => state = EmulatorState::Paused,
                            EmulatorCommand::Run => {
                                // on run, ensure that we _always_ run at least one instruction
                                let old_pc = cpu.get_executing_pc();
                                while cpu.get_executing_pc() == old_pc {
                                    cpu.fetch_decode_execute();
                                }
                                state = EmulatorState::Running
                            }
                            EmulatorCommand::Step(count) => {
                                for _ in 0..count {
                                    cpu.fetch_decode_execute();
                                }

                                state = EmulatorState::Paused
                            }
                            EmulatorCommand::LoadRom(path) => {
                                let file = match File::open(path) {
                                    Ok(file) => file,
                                    Err(e) => {
                                        println!("{e:?}");
                                        continue;
                                    }
                                };

                                let cartridge = match Cartridge::new(file, None) {
                                    Ok(cart) => cart,
                                    Err(e) => {
                                        println!("{e:?}");
                                        continue;
                                    }
                                };

                                cpu = Cpu::new(cartridge);
                            }
                            EmulatorCommand::KeyPressed(key) => {
                                cpu.bus.keypad.set_pressed(key, true)
                            }
                            EmulatorCommand::KeyReleased(key) => {
                                cpu.bus.keypad.set_pressed(key, false)
                            }
                            EmulatorCommand::CreateNewSaveState => {
                                let new_save_state = cpu.clone();
                                save_states.push(new_save_state);
                                num_save_states.fetch_add(1, Ordering::SeqCst);
                            }
                            EmulatorCommand::UpdateSaveState(idx) => {
                                if idx > save_states.len() {
                                    panic!("got a request to update save state at index {}, but only have {} indices available", idx, save_states.len());
                                }

                                let new_save_state = cpu.clone();
                                save_states[idx] = new_save_state;
                            }
                            EmulatorCommand::LoadSaveState(idx) => {
                                if idx > save_states.len() {
                                    panic!("got a request to load save state at index {}, but only have {} indices available", idx, save_states.len());
                                }

                                cpu = save_states[idx].clone();
                            }
                        }
                    }

                    match state {
                        EmulatorState::Running => {
                            let cycle_start = cpu.bus.cycle_count();
                            'frame_loop: while (cpu.bus.cycle_count() - cycle_start)
                                < (CYCLES_PER_SECOND / 60)
                            {
                                for breakpoint in breakpoints.lock().unwrap().iter_mut() {
                                    if breakpoint.active
                                        && breakpoint.address == cpu.get_executing_pc()
                                    {
                                        state = EmulatorState::Paused;
                                        break 'frame_loop; // if we hit a breakpoint, immediately stop executing for this frame
                                    }
                                }
                                cpu.fetch_decode_execute();
                            }
                        }
                        EmulatorState::Paused => {}
                    }

                    {
                        display_buffer
                            .lock()
                            .unwrap()
                            .copy_from_slice(cpu.bus.lcd.get_buffer());

                        {
                            let executing_pc = cpu.get_executing_pc();

                            let mut disassembly_info_lock = disassembly_info.lock().unwrap();
                            disassembly_info_lock.pc = executing_pc;
                        }

                        {
                            const REGISTERS_TO_READ: &[Register] = &[
                                Register::R0,
                                Register::R1,
                                Register::R2,
                                Register::R3,
                                Register::R4,
                                Register::R5,
                                Register::R6,
                                Register::R7,
                                Register::R8,
                                Register::R9,
                                Register::R10,
                                Register::R11,
                                Register::R12,
                                Register::R13,
                                Register::R14,
                                Register::R15,
                            ];

                            let register_values = REGISTERS_TO_READ
                                .iter()
                                .map(|&register| {
                                    let value = cpu.read_register(register, |pc| pc);
                                    RegisterValue { register, value }
                                })
                                .collect::<Box<_>>();

                            *registers_info.lock().unwrap() = register_values;
                        }

                        {
                            let new_cpu_info = CpuInfo {
                                sign_flag: cpu.get_sign_flag(),
                                zero_flag: cpu.get_zero_flag(),
                                carry_flag: cpu.get_carry_flag(),
                                overflow_flag: cpu.get_overflow_flag(),
                                irq_disable: cpu.get_irq_disable(),
                                fiq_disable: cpu.get_fiq_disable(),
                                instruction_mode: cpu.get_instruction_mode(),
                                cpu_mode: cpu.get_cpu_mode(),
                                irq_buffer: cpu.bus.get_interrupt_request_debug(),
                                open_bus_data: cpu.bus.open_bus_data,
                            };
                            *cpu_info.lock().unwrap() = new_cpu_info;
                        }
                    }

                    {
                        let timer_infos = cpu
                            .bus
                            .timers
                            .iter()
                            .map(|timer| TimerInfo {
                                counter: timer.get_current_counter(),
                                reload: timer.get_current_reload(),
                            })
                            .collect::<Box<[_]>>();

                        *timer_info.lock().unwrap() = timer_infos;
                    }
                    cycles_executed.store(cpu.bus.cycle_count(), Ordering::SeqCst);
                }
            });
        }

        Self {
            display_buffer,
            emulator_command_sender,
            step_count: 1,
            cycles_executed,
            disassembly_info,
            registers_info,
            cpu_info,
            timer_info,
            breakpoints,
            num_save_states,
        }
    }
}

impl MyEguiApp {
    fn controls(&mut self, ui: &mut Ui) {
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

        if ui.button("Choose ROM").clicked() {
            let sender = self.emulator_command_sender.clone();
            thread::spawn(move || {
                if let Some(file) = FileDialog::new()
                    .add_filter("GBA ROM", &["gba"])
                    .pick_file()
                {
                    sender.send(EmulatorCommand::LoadRom(file)).unwrap();
                } else {
                    println!("user cancelled file selection");
                }
            });
        }

        if ui.button("Create Save State").clicked() {
            self.emulator_command_sender
                .send(EmulatorCommand::CreateNewSaveState)
                .unwrap();
        }

        egui::CollapsingHeader::new("Save States")
            .default_open(true)
            .show(ui, |ui| {
                for i in 0..self.num_save_states.load(Ordering::SeqCst) {
                    ui.horizontal(|ui| {
                        ui.label(format!("Save State {}", i));
                        if ui.button("Load").clicked() {
                            self.emulator_command_sender
                                .send(EmulatorCommand::LoadSaveState(i))
                                .unwrap();
                        }

                        if ui.button("Save").clicked() {
                            self.emulator_command_sender
                                .send(EmulatorCommand::UpdateSaveState(i))
                                .unwrap();
                        }
                    });
                }
            });
    }

    fn emulator_window(&self, ui: &mut Ui) {
        let rgb_data = self
            .display_buffer
            .lock()
            .unwrap()
            .iter()
            .flat_map(|row| {
                row.iter().flat_map(|pixel| {
                    let red = (pixel.red() << 3) | (pixel.red() >> 2);
                    let green = (pixel.green() << 3) | (pixel.green() >> 2);
                    let blue = (pixel.blue() << 3) | (pixel.blue() >> 2);
                    [red, green, blue]
                })
            })
            .collect::<Vec<_>>();

        let image = ColorImage::from_rgb([Lcd::LCD_WIDTH, Lcd::LCD_HEIGHT], &rgb_data);
        let texture = ui
            .ctx()
            .load_texture("gba-texture", image, TextureOptions::NEAREST);

        ui.image(texture.id(), ui.available_size());
    }

    fn register_info(&self, ui: &mut Ui) {
        let registers_info_lock = self.registers_info.lock().unwrap();
        for register_info in registers_info_lock.iter() {
            ui.horizontal(|ui| {
                ui.label(&format!("{}", register_info.register));
                ui.add(
                    TextEdit::singleline(&mut format!("{:08X}", register_info.value))
                        .interactive(false),
                );
            });
        }
    }

    fn cpu_info(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("CPU Cycles");
            ui.add(
                TextEdit::singleline(&mut format!(
                    "{}",
                    self.cycles_executed.load(Ordering::SeqCst)
                ))
                .interactive(false),
            );
        });

        let cpu_info_lock = self.cpu_info.lock().unwrap();
        let info_fields: [(&str, &dyn Debug); 8] = [
            ("sign flag", &cpu_info_lock.sign_flag),
            ("zero flag", &cpu_info_lock.zero_flag),
            ("carry flag", &cpu_info_lock.carry_flag),
            ("overflow flag", &cpu_info_lock.overflow_flag),
            ("irq disable", &cpu_info_lock.irq_disable),
            ("fiq disable", &cpu_info_lock.fiq_disable),
            ("instruction mode", &cpu_info_lock.instruction_mode),
            ("cpu mode", &cpu_info_lock.cpu_mode),
        ];

        for (name, value) in info_fields {
            ui.horizontal(|ui| {
                ui.label(name.to_string());
                ui.add(TextEdit::singleline(&mut format!("{:?}", value)).interactive(false));
            });
        }

        ui.horizontal(|ui| {
            ui.label("open bus");
            ui.add(
                TextEdit::singleline(&mut format!("{:08X}", cpu_info_lock.open_bus_data))
                    .interactive(false),
            );
        });

        CollapsingHeader::new("Irq Sync Buffer")
            .default_open(true)
            .show(ui, |ui| {
                for (i, irq_val) in cpu_info_lock.irq_buffer.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Idx {}", i));
                        ui.add(
                            TextEdit::singleline(&mut format!("{:08b}", irq_val))
                                .interactive(false),
                        );
                    });
                }
            });

        CollapsingHeader::new("Timers")
            .default_open(true)
            .show(ui, |ui| {
                let timer_info_lock = self.timer_info.lock().unwrap();

                for (i, info) in timer_info_lock.iter().enumerate() {
                    ui.collapsing(format!("Timer {}", i), |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Counter");
                            ui.add(
                                TextEdit::singleline(&mut format!("{:04X}", info.counter))
                                    .interactive(false),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Reload");
                            ui.add(
                                TextEdit::singleline(&mut format!("{}", info.reload))
                                    .interactive(false),
                            );
                        });
                    });
                }
            });
    }

    fn debugger(&mut self, ui: &mut Ui) {
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

        CollapsingHeader::new("Breakpoints")
            .default_open(true)
            .show(ui, |ui| {
                let mut breakpoints_lock = self.breakpoints.lock().unwrap();

                for breakpoint in breakpoints_lock.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.add(
                            Slider::new(&mut breakpoint.address, 0..=0xFFFF_FFFF)
                                .hexadecimal(8, false, true),
                        );
                        ui.checkbox(&mut breakpoint.active, "Active");

                        let mut stopped_at =
                            breakpoint.address == self.disassembly_info.lock().unwrap().pc;
                        ui.checkbox(&mut stopped_at, "Stopped");
                    });
                }

                if ui.button("Add Breakpoint").clicked() {
                    breakpoints_lock.push(BreakpointInfo::default());
                }
            });
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::Window::new("Controls").show(ctx, |ui| self.controls(ui));

        egui::Window::new("Emulator Window")
            .collapsible(false)
            .default_height(Lcd::LCD_HEIGHT as f32 * 4.0)
            .default_width(Lcd::LCD_WIDTH as f32 * 4.0)
            .show(ctx, |ui| self.emulator_window(ui));

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

        egui::Window::new("Register Viewer").show(ctx, |ui| self.register_info(ui));
        egui::Window::new("CPU Info").show(ctx, |ui| self.cpu_info(ui));
        egui::Window::new("Debugger").show(ctx, |ui| self.debugger(ui));
    }
}
