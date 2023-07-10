mod sample_source;

use sample_source::sample_source;

use std::time::Duration;
use std::{fs::File, time::Instant};

use anyhow::{anyhow, Result};
use clap::Parser;
use pixels::{wgpu::TextureFormat, PixelsBuilder, SurfaceTexture};
use rodio::{OutputStream, Sink};
use winit::event_loop::EventLoop;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    window::WindowBuilder,
};

use emulator_core::{calculate_lcd_checksum, Cartridge, Cpu, Key, Lcd, CYCLES_PER_SECOND};

const APU_SAMPLE_RATE: u32 = 44_100;
const FPS_TARGET: u32 = 60;

#[derive(Debug, Parser)]
struct Args {
    rom: String,

    #[clap(short, long)]
    frames: Option<u64>,

    #[clap(long)]
    limit_framerate: bool,
}

#[allow(unused)]
fn press_key(cpu: &mut Cpu, key: Key) {
    cpu.bus.keypad.set_pressed(key, true);
    for _ in 0..500_000 {
        cpu.fetch_decode_execute();
    }
    cpu.bus.keypad.set_pressed(key, false);
    for _ in 0..500_000 {
        cpu.fetch_decode_execute();
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let (mut source_sender, source) = sample_source(APU_SAMPLE_RATE);
    sink.append(source);

    let args = Args::parse();

    let save_file_name = format!("{}.sav", args.rom);

    let rom_file =
        File::open(&args.rom).map_err(|_| anyhow!("failed to open ROM file \"{}\"", args.rom))?;

    let save_file = File::open(&save_file_name).ok();

    log::info!("attempting to read save info from {save_file_name}");
    let save_data = save_file.map(serde_cbor::from_reader).transpose()?;

    match save_data {
        Some(_) => log::info!("successfuly read save info from {save_file_name}"),
        None => log::info!("failed to read save info from {save_file_name}"),
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Quantatic's GBA Emulator")
        .build(&event_loop)?;

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(
            Lcd::LCD_WIDTH.try_into().unwrap(),
            Lcd::LCD_HEIGHT.try_into().unwrap(),
            surface_texture,
        )
        .texture_format(TextureFormat::Rgba8UnormSrgb)
        .enable_vsync(true)
        .build()?
    };

    let cartridge = Cartridge::new(rom_file, save_data)?;
    let mut cpu = Cpu::new(cartridge);

    let init = Instant::now();
    let mut last_frame = Instant::now();
    let mut i = 0;
    let mut apu_samples: u64 = 0;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                let cycle_start = cpu.bus.cycle_count();
                let mut apu_samples = 0;
                loop {
                    let cycles_elapsed = cpu.bus.cycle_count() - cycle_start;

                    cpu.fetch_decode_execute();

                    while cycles_elapsed
                        > (apu_samples * CYCLES_PER_SECOND / u64::from(APU_SAMPLE_RATE))
                    {
                        let sample = cpu.sample_apu();
                        source_sender.push(sample[0]);
                        source_sender.push(sample[1]);
                        apu_samples += 1;
                    }

                    if cycles_elapsed >= (CYCLES_PER_SECOND / 60) {
                        break;
                    }
                }

                let draw_buffer = pixels.frame_mut();
                let lcd_buffer = cpu.bus.lcd.get_buffer();
                for (index, pixel) in lcd_buffer.iter().flatten().enumerate() {
                    draw_buffer[(index * 4)..][0] = (pixel.red() << 3) | (pixel.red() >> 2);
                    draw_buffer[(index * 4)..][1] = (pixel.green() << 3) | (pixel.green() >> 2);
                    draw_buffer[(index * 4)..][2] = (pixel.blue() << 3) | (pixel.blue() >> 2);
                    draw_buffer[(index * 4)..][3] = 255;
                }
                pixels.render().expect("failed to render new frame");

                if args.limit_framerate {
                    while last_frame.elapsed() < Duration::from_secs(1) / FPS_TARGET {
                        std::thread::yield_now();
                    }
                }

                let time_elapsed = last_frame.elapsed();
                let fps = 1.0 / time_elapsed.as_secs_f64();
                window.set_title(format!("FPS: {}", fps).as_str());

                last_frame = Instant::now();
                match args.frames {
                    Some(frames) if i >= frames => *control_flow = ControlFlow::Exit,
                    _ => {}
                };

                i += 1;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                window_id,
            } if window_id == window.id() => {
                pixels
                    .resize_surface(new_size.width, new_size.height)
                    .unwrap();
                log::info!("resized to ({}, {})", new_size.width, new_size.height);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(keycode),
                                state,
                                ..
                            },
                        is_synthetic: false,
                        ..
                    },
                window_id,
            } if window_id == window.id() => {
                let pressed = match state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                };

                match keycode {
                    VirtualKeyCode::Z => cpu.bus.keypad.set_pressed(Key::B, pressed),
                    VirtualKeyCode::X => cpu.bus.keypad.set_pressed(Key::A, pressed),
                    VirtualKeyCode::RShift | VirtualKeyCode::LShift => {
                        cpu.bus.keypad.set_pressed(Key::Select, pressed)
                    }
                    VirtualKeyCode::Return => cpu.bus.keypad.set_pressed(Key::Start, pressed),
                    VirtualKeyCode::Up => cpu.bus.keypad.set_pressed(Key::Up, pressed),
                    VirtualKeyCode::Down => cpu.bus.keypad.set_pressed(Key::Down, pressed),
                    VirtualKeyCode::Left => cpu.bus.keypad.set_pressed(Key::Left, pressed),
                    VirtualKeyCode::Right => cpu.bus.keypad.set_pressed(Key::Right, pressed),
                    VirtualKeyCode::Q => cpu.bus.keypad.set_pressed(Key::L, pressed),
                    VirtualKeyCode::E => cpu.bus.keypad.set_pressed(Key::R, pressed),
                    VirtualKeyCode::Space if pressed => {
                        log::error!("current checksum: {:016X}", calculate_lcd_checksum(&cpu));
                    }
                    _ => {}
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::LoopDestroyed => {
                log::info!("ran for {:?}", init.elapsed());

                let save_file_name = format!("{}.sav", args.rom);
                log::info!("writing save data to {save_file_name}");
                let save_file = File::create(&save_file_name).expect("failed to create save file");
                serde_cbor::to_writer(save_file, cpu.bus.cartridge.get_backup())
                    .expect("failed to write save data to save file");
                log::info!("finished writing save data to {save_file_name}");
            }
            _ => {}
        };
    });
}
