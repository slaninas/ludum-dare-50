use pixels::{Pixels, SurfaceTexture};

use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use winit_input_helper::WinitInputHelper;

use rustbitmap::bitmap::image::BitMap;
use rustbitmap::bitmap::rgba::Rgba;

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

    let mut blocks = vec![
        BitMap::read("test.bmp").unwrap(),
        BitMap::read("test3.bmp").unwrap(),
    ];

    let img = BitMap::read("img.bmp").unwrap();
    let mut current_block = 1;
    let mut next_block = 1;

    let mut horizontal_shift = 0f32;

    let mut player = Player::new();
    let mut score: u64 = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            } else if input.key_pressed(VirtualKeyCode::C) || input.key_held(VirtualKeyCode::C) {
                player.jump();
            }
        }

        match event {
            Event::MainEventsCleared => {
                score += 1;
                let alive = player.update(&blocks, (current_block, next_block), horizontal_shift);
                if !alive {
                    panic!("RIP");
                }
                clear(pixels.get_frame());

                draw_tiles(
                    pixels.get_frame(),
                    &blocks,
                    (current_block, next_block),
                    horizontal_shift as u32,
                    &img,
                );
                player.draw(pixels.get_frame());

                draw_score(score, &img, pixels.get_frame());

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
    });
}

fn draw_score(score: u64, img: &BitMap, pixels: &mut [u8]) {
    let numericals = (score as f64).log10() as u64 + 1;
    let end = 22 * TILE_SCALE as u64;

    for i in 0..numericals {
        let remainer = (score / 10u64.pow(i as u32)) % 10;
        let tile = img.crop((100 + 10 * remainer) as u32, 0, (100 + 10 * (remainer + 1)) as u32, 10).unwrap();


        draw_tile(pixels, &tile, (end as i32 - 10 * i as i32, 10 as i32));

    }

}

struct Player {
    pos_x: f32,
    pos_y: f32,
    speed_y: f32,
    size_x: u32,
    size_y: u32,

    jump_info: JumpInfo,
    tile: BitMap,
}

struct JumpInfo {
    on_ground: bool,
    jumping: bool,
    jump_start: Instant,
}

impl Player {
    fn new() -> Self {
        let tile = BitMap::read("img.bmp").unwrap();
        let tile = tile.crop(30, 0, 40, 10).unwrap();
        Player {
            pos_x: 50.0,
            pos_y: 80.0,
            speed_y: 0.0,
            size_x: 10,
            size_y: 10,

            jump_info: JumpInfo {
                on_ground: false,
                jumping: false,
                jump_start: Instant::now(),
            },
            tile,
        }
    }

    fn jump(&mut self) {
        if self.jump_info.on_ground {
            self.jump_info.jumping = true;
            self.jump_info.jump_start = Instant::now();
            self.speed_y -= 2.0;
        } else if self.jump_info.jumping {
            let elapsed = self.jump_info.jump_start.elapsed().as_millis();
            if elapsed < 250 {
                self.speed_y -= 0.2;
            }
        }
    }

    fn update(
        &mut self,
        blocks: &Vec<BitMap>,
        blocks_ids: (u32, u32),
        horizontal_shift: f32,
    ) -> bool {
        let steps = 5;
        let speed_y_fraction = self.speed_y / steps as f32;

        self.pos_x -= 0.01;

        if self.pos_x <= 0.0 {
            return false;
        }

        if self.pos_x < 30.0 {
            self.pos_y += 3.0;
            self.pos_x -= 3.0;
            return true;
        }

        let mut vertical_hit = false;
        for step in 0..steps {
            self.pos_y += speed_y_fraction;

            // Vertical
            let bottom_left = (self.pos_x as u32, self.pos_y as u32 + self.size_y);
            let bottom_right = (
                self.pos_x as u32 + self.size_x,
                self.pos_y as u32 + self.size_y,
            );

            let bottom_left_pixel =
                get_pixel(bottom_left, blocks, blocks_ids, horizontal_shift as u32);
            let bottom_right_pixel =
                get_pixel(bottom_right, blocks, blocks_ids, horizontal_shift as u32);

            if (is_solid(&bottom_left_pixel)
                || is_solid(&bottom_right_pixel) )&& self.speed_y >= 0.0
            {
                self.pos_y -= speed_y_fraction;
                self.speed_y = 0.0;
                self.jump_info.on_ground = true;
                self.jump_info.jumping = false;

                if same_rgb(&bottom_left_pixel, &Rgb::new(99, 155, 255))
                    || same_rgb(&bottom_right_pixel, &Rgb::new(99, 155, 255))
                {
                    self.pos_x += 5.0;
                }
                else if same_rgb(&bottom_left_pixel, &Rgb::new(217, 87, 99))
                    || same_rgb(&bottom_right_pixel, &Rgb::new(217, 87, 99))
                {
                    self.pos_x -= 5.0;
                    self.speed_y -= 1.5;
                }

                break;
            } else {
                self.jump_info.on_ground = false;
                self.speed_y += 0.15 / steps as f32;
            }
        }

        return true;
    }

    fn draw(&self, pixels: &mut [u8]) {
        draw_tile(pixels, &self.tile, (self.pos_x as i32, self.pos_y as i32));
    }
}

fn is_solid(rgb: &Rgb) -> bool {
    same_rgb(&rgb, &Rgb::new(255, 255, 255)) || same_rgb(&rgb, &Rgb::new(99, 155, 255))
}

fn same_rgb(rgb: &Rgb, rgb2: &Rgb) -> bool {
    rgb.red == rgb2.red && rgb.green == rgb2.green && rgb.blue == rgb2.blue
}

fn get_block_color(block: &BitMap, x: u32, y: u32) -> Rgb {
    let pixel = block.get_pixel(x / TILE_SCALE, y / TILE_SCALE).unwrap();
    Rgb::from_rgba(&pixel)
}

fn get_pixel(
    coords: (u32, u32),
    blocks: &Vec<BitMap>,
    blocks_ids: (u32, u32),
    horizontal_shift: u32,
) -> Rgb {
    let (x, y) = coords;
    let y_inverted = HEIGHT - y - 1;
    let index = (y * WIDTH + x) as usize;
    let index_inverted = (y_inverted * WIDTH + x) as usize;

    let pixel = if horizontal_shift + x < HORIZONTAL_TILES * TILE_SCALE {
        get_block_color(&blocks[blocks_ids.0 as usize], x + horizontal_shift, y)
    } else {
        get_block_color(
            &blocks[blocks_ids.1 as usize],
            horizontal_shift - HORIZONTAL_TILES * TILE_SCALE + x,
            y,
        )
    };

    pixel
}

struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

impl Rgb {
    fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    fn from_rgba(rgba: &rustbitmap::bitmap::rgba::Rgba) -> Self {
        Self {
            red: rgba.get_red(),
            green: rgba.get_green(),
            blue: rgba.get_blue(),
        }
    }
}

fn clear(pixels: &mut [u8]) {
    for i in 0..pixels.len() / 4 {
        pixels[4 * i + 0] = 175;
        pixels[4 * i + 1] = 175;
        pixels[4 * i + 2] = 175;
        pixels[4 * i + 3] = 255;
    }
}

fn draw_tile(pixels: &mut [u8], tile: &BitMap, coords: (i32, i32)) {
    let (start_x, start_y) = coords;

    for y in start_y..start_y + 10 {
        if y < 0 || y >= HEIGHT as i32 {
            continue;
        }
        for x in start_x..start_x + 10 {
            if x < 0 || x >= WIDTH as i32 {
                continue;
            }

            let surface_index = y * WIDTH as i32 + x;
            let tile_pixel = tile
                .get_pixel(x as u32 - start_x as u32, 9 - (y as u32 - start_y as u32))
                .unwrap();
            if tile_pixel.get_red() == 132
                && tile_pixel.get_green() == 126
                && tile_pixel.get_blue() == 135
            {
                continue;
            }
            pixels[4 * surface_index as usize + 0] = tile_pixel.get_red();
            pixels[4 * surface_index as usize + 1] = tile_pixel.get_green();
            pixels[4 * surface_index as usize + 2] = tile_pixel.get_blue();
            pixels[4 * surface_index as usize + 3] = 255;
        }
    }
}

fn draw_tiles(
    pixels: &mut [u8],
    blocks: &Vec<BitMap>,
    blocks_ids: (u32, u32),
    horizontal_shift: u32,
    img: &BitMap,
) {
    let tile = img.crop(0, 0, 10, 10).unwrap();
    let spike = img.crop(10, 0, 20, 10).unwrap();
    let speedup = img.crop(20, 0, 30, 10).unwrap();
    let mut first_block = true;

    loop {
        let block = if first_block {
            &blocks[blocks_ids.0 as usize]
        } else {
            &blocks[blocks_ids.1 as usize]
        };

        for y in 0..16 {
            for x in 0..48 {
                let pixel = Rgb::from_rgba(block.get_pixel(x, y).unwrap());
                let xx = if first_block {
                    x as i32 * TILE_SCALE as i32 - horizontal_shift as i32
                } else {
                    x as i32 * TILE_SCALE as i32
                        + (HORIZONTAL_TILES as i32 * TILE_SCALE as i32 - horizontal_shift as i32)
                };
                let mut yy = y as i32 * TILE_SCALE as i32;

                if xx < 25 {
                    yy += 25 - xx;
                }

                if same_rgb(&pixel, &Rgb::new(255, 255, 255)) {
                    draw_tile(pixels, &tile, (xx, yy));
                } else if same_rgb(&pixel, &Rgb::new(217, 87, 99)) {
                    draw_tile(pixels, &spike, (xx, yy));
                } else if same_rgb(&pixel, &Rgb::new(99, 155, 255)) {
                    draw_tile(pixels, &speedup, (xx, yy));
                }
            }
        }
        if first_block {
            first_block = false;
        } else {
            break;
        }
    }
}
