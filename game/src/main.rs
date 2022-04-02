use pixels::{Pixels, SurfaceTexture};

use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use winit_input_helper::WinitInputHelper;

use rustbitmap::bitmap::image::BitMap;

use rand::{rngs::ThreadRng, Rng};
use std::{
    thread,
    time::{Duration, Instant},
};

// Starting with GBA resolution
const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;
fn main() {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 4.0, HEIGHT as f64 * 4.0);

        WindowBuilder::new()
            .with_title("Ludum Dare #50")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
    };

    let mut rng = rand::thread_rng();

    let mut last_update = Instant::now();
    let frame_time = (1000.0 / 60.0) as i16;

    let block = load_block();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                // println!("draw");
                draw(pixels.get_frame(), &mut rng, &block);

                let elapsed = last_update.elapsed();
                let diff = frame_time - elapsed.as_millis() as i16;

                if diff > 0 {
                    // println!("sleeping for {} ms", diff);
                    thread::sleep(Duration::from_millis(diff as u64));
                }

                last_update = Instant::now();

                if pixels.render().map_err(|e| {}).is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            _ => {}
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
    });
}

fn load_block() -> BitMap {
    BitMap::read("test.bmp").unwrap()
}

fn draw(pixels: &mut [u8], rng: &mut ThreadRng, block: &BitMap) {
    let total_pixels = pixels.len();
    let block_pixels = block.get_pixels();
    assert!(total_pixels == 4 * block_pixels.len());

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let y_inverted = HEIGHT - y - 1;
            let index = (y * WIDTH + x) as usize;
            let index_inverted = (y_inverted * WIDTH + x) as usize;

            pixels[4 * index + 0] = block_pixels[index_inverted].get_red();
            pixels[4 * index + 1] = block_pixels[index_inverted].get_green();
            pixels[4 * index + 2] = block_pixels[index_inverted].get_blue();
            pixels[4 * index + 3] = 255 as u8;
        }
    }
}
