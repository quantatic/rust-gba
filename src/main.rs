mod apu;
mod bit_manipulation;
mod bus;
mod cpu;
mod data_access;
mod keypad;
mod lcd;
mod timer;

use std::error::Error;

use lazy_static::lazy_static;
use pixels::{wgpu::TextureFormat, PixelsBuilder, SurfaceTexture};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use bit_manipulation::BitManipulation;
use data_access::DataAccess;

use crate::keypad::Key;

#[cfg(target_pointer_width = "16")]
compile_error!("architecture with pointer size >= 32 required");

const DEBUG_AND_PANIC_ON_LOOP: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", std::mem::size_of::<cpu::Cpu>());

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Quantatic's GBA Emulator")
        .build(&event_loop)?;

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(
            lcd::LCD_WIDTH.try_into().unwrap(),
            lcd::LCD_HEIGHT.try_into().unwrap(),
            surface_texture,
        )
        .texture_format(TextureFormat::Rgba8UnormSrgb)
        .enable_vsync(true)
        .build()?
    };

    let mut cpu = cpu::Cpu::default();

    // for _ in 0..74_000_000 {
    //     cpu.fetch_decode_execute(false);
    // }

    // let mut enter_pressed = false;
    // loop {
    //     for _ in 0..100_000 {
    //         cpu.fetch_decode_execute(true);
    //     }

    //     cpu.bus.keypad.set_pressed(Key::Start, enter_pressed);
    //     enter_pressed = !enter_pressed;
    // }

    let mut i = 0;
    event_loop.run(move |event, _, _control_flow| {
        match event {
            Event::MainEventsCleared => {
                for _ in 0..266_666 {
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
                    _ => {}
                }
            }
            _ => {}
        };
    });
}
