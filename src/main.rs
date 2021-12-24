use std::error::Error;

use pixels::{wgpu::TextureFormat, PixelsBuilder, SurfaceTexture};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

mod apu;
mod bit_manipulation;
mod bus;
mod cpu;
mod lcd;

#[cfg(target_pointer_width = "16")]
compile_error!("architecture with pointer size >= 32 required");

const DEBUG_AND_PANIC_ON_LOOP: bool = true;

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Aidan's GBA Emulator")
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

    for _ in 0..74_000_000 {
        cpu.fetch_decode_execute(false);
    }

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                for _ in 0..266_666 {
                    cpu.fetch_decode_execute(DEBUG_AND_PANIC_ON_LOOP);
                }

                let draw_buffer = pixels.get_frame();
                let lcd_buffer = cpu.bus.lcd.buffer();
                for (index, pixel) in lcd_buffer.iter().flatten().enumerate() {
                    draw_buffer[(index * 4)..][0] = pixel.red;
                    draw_buffer[(index * 4)..][1] = pixel.green;
                    draw_buffer[(index * 4)..][2] = pixel.blue;
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
            _ => {}
        };
    });
}
