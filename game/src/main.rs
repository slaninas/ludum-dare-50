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
const TILE_SCALE: u32 = 10;
const HORIZONTAL_TILES: u32 = 48;

fn main() {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 6.0, HEIGHT as f64 * 6.0);

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

    let mut blocks = vec![BitMap::read("test3.bmp").unwrap(), BitMap::read("test2.bmp").unwrap()];
    let mut current_block = 0;
    let mut next_block = 0;

    let mut horizontal_shift = 0f32;

    let mut player = Player::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                player.update();
                draw_tiles(
                    pixels.get_frame(),
                    &blocks,
                    (current_block, next_block),
                    horizontal_shift as u32,
                );
                player.draw(pixels.get_frame());

                let elapsed = last_update.elapsed();
                let diff = frame_time - elapsed.as_millis() as i16;

                if diff > 0 {
                    println!("sleeping for: {} ms", diff);
                    thread::sleep(Duration::from_millis(diff as u64));
                }

                last_update = Instant::now();

                if pixels.render().map_err(|e| {}).is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                horizontal_shift += 2.;
                if horizontal_shift >= (HORIZONTAL_TILES * TILE_SCALE) as f32 {
                    horizontal_shift = 0.0;
                    let tmp = current_block;
                    current_block = next_block;
                    next_block = tmp;
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

struct Player {
    pos_x: f32,
    pos_y: f32,
    speed_y: f32,
    size_x: u16,
    size_y: u16,
}

impl Player {
    fn new() -> Self {
        Player {
            pos_x: 40.0,
            pos_y: 80.0,
            speed_y: 0.0,
            size_x: 10,
            size_y: 10,
        }
    }

    fn update(&mut self) {
        self.pos_y += self.speed_y;
        self.speed_y += 0.05;
    }

    fn draw(&self, pixels: &mut [u8]) {

        let index = (self.pos_y as usize * WIDTH as usize + self.pos_x as usize) as usize;
        println!("pos {} {}, WIDTH {}", self.pos_x, self.pos_y, WIDTH);
        println!("index {}", index);
        for y in 0..self.size_y as usize {
            for x in 0..self.size_x as usize {
                pixels[4 * (index + y * WIDTH as usize + x) + 0] = 255;
                pixels[4 * (index + y * WIDTH as usize + x) + 1] = 0;
                pixels[4 * (index + y * WIDTH as usize + x) + 2] = 0;
                pixels[4 * (index + y * WIDTH as usize + x) + 3] = 255;
            }
        }

    }
}

fn get_block_color(block: &BitMap, x: u32, y: u32) -> [u8; 4] {
    let pixel = block.get_pixel(x / TILE_SCALE, y / TILE_SCALE).unwrap();
    [
        pixel.get_red(),
        pixel.get_green(),
        pixel.get_blue(),
        pixel.get_alpha(),
    ]
}

fn draw_tiles(pixels: &mut [u8], blocks: &Vec<BitMap>, blocks_ids: (u32, u32), horizontal_shift: u32) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let y_inverted = HEIGHT - y - 1;
            let index = (y * WIDTH + x) as usize;
            let index_inverted = (y_inverted * WIDTH + x) as usize;

            let block_pixel = if horizontal_shift + x < HORIZONTAL_TILES * TILE_SCALE  {
                get_block_color(&blocks[blocks_ids.0 as usize], x + horizontal_shift, y)
            } else {
                get_block_color(&blocks[blocks_ids.1 as usize], horizontal_shift - HORIZONTAL_TILES * TILE_SCALE + x, y)
            };

            pixels[4 * index + 0] = block_pixel[0];
            pixels[4 * index + 1] = block_pixel[1];
            pixels[4 * index + 2] = block_pixel[2];
            pixels[4 * index + 3] = 255 as u8;
        }
    }
}
