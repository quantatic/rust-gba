use std::{fs::File, time::Instant};

use anyhow::{anyhow, Result};
use clap::Parser;
use pixels::{wgpu::TextureFormat, PixelsBuilder, SurfaceTexture};
use winit::event_loop::EventLoop;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    window::WindowBuilder,
};

use emulator_core::{calculate_lcd_checksum, Cartridge, Cpu, Key, Lcd, CYCLES_PER_SECOND};

const DEBUG_AND_PANIC_ON_LOOP: bool = false;

#[derive(Debug, Parser)]
struct Args {
    rom: String,

    #[clap(short, long)]
    frames: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let rom_file =
        File::open(&args.rom).map_err(|_| anyhow!("failed to open ROM file \"{}\"", args.rom))?;

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

    let cartridge = Cartridge::new(rom_file);
    let mut cpu = Cpu::new(cartridge);

    // for _ in 0..74_500_000 {
    //     cpu.fetch_decode_execute(false);
    // }

    let init = Instant::now();
    let mut last_step = Instant::now();
    let mut i = 0;
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                for _ in 0..(CYCLES_PER_SECOND / 60) {
                    cpu.fetch_decode_execute(DEBUG_AND_PANIC_ON_LOOP);
                }

                let draw_buffer = pixels.get_frame();
                let lcd_buffer = cpu.bus.lcd.get_buffer();
                for (index, pixel) in lcd_buffer.iter().flatten().enumerate() {
                    draw_buffer[(index * 4)..][0] = (pixel.red << 3) | (pixel.red >> 2);
                    draw_buffer[(index * 4)..][1] = (pixel.green << 3) | (pixel.green >> 2);
                    draw_buffer[(index * 4)..][2] = (pixel.blue << 3) | (pixel.blue >> 2);
                    draw_buffer[(index * 4)..][3] = 255;
                }
                pixels.render().expect("failed to render new frame");

                let time_elapsed = last_step.elapsed();
                let fps = 1.0 / time_elapsed.as_secs_f64();
                window.set_title(format!("FPS: {}", fps).as_str());

                last_step = Instant::now();
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
                pixels.resize_surface(new_size.width, new_size.height);
                println!("resized to ({}, {})", new_size.width, new_size.height);
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
                        println!("current checksum: {:016X}", calculate_lcd_checksum(&cpu));
                    }
                    _ => {}
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::LoopDestroyed => println!("ran for {:?}", init.elapsed()),
            _ => {}
        };
    });
}