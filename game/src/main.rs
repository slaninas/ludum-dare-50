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

use rodio::source::{SineWave, Source};
use rodio::{Decoder, OutputStream, Sink};

use rand::{rngs::ThreadRng, Rng};
use std::{
    fs::File,
    io::{BufReader, Write},
    thread,
    time::{Duration, Instant},
};

// Starting with GBA resolution
const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;
const TILE_SCALE: u32 = 10;
const HORIZONTAL_TILES: u32 = 48;
const MAX_PLAYER_X: u32 = 59;
const MIN_PLAYER_X: u32 = 30;

#[derive(Debug)]
enum State {
    SPLASH,
    RUNNING,
    GAMEOVER,
}

#[derive(Debug)]
enum Update {
    NOTHING,
    SPEEDUP,
    DEAD,
}

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
            .with_resizable(false)
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

    let max_blocks = 1;
    let mut blocks = load_blocks(max_blocks);

    let img = BitMap::read("img.bmp").unwrap();
    let splash_img = BitMap::read("splash.bmp").unwrap();
    let gameover_img = BitMap::read("gameover.bmp").unwrap();
    let mut current_block = 0;
    let mut next_block = get_next_block(max_blocks);

    let mut horizontal_shift = 0f32;

    let mut player = Player::new();


    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let source = SineWave::new(440.0)
        .take_duration(Duration::from_secs_f32(1.25))
        .amplify(0.20);
    let mut last_played = Instant::now();
    let mut last_speedup_sound = Instant::now();

    let mut highscore = get_highscore();
    let mut score: u64 = 0;

    let mut state = State::SPLASH;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                save_highscore(std::cmp::max(score, highscore));
                return;
            } else if input.key_pressed(VirtualKeyCode::X) || input.key_held(VirtualKeyCode::X) {
                match &state {
                    State::SPLASH => {
                        state = State::RUNNING;
                        score = 0;
                    }
                    State::RUNNING => player.jump(),
                    State::GAMEOVER => {
                        state = State::RUNNING;
                        score = 0;
                        player = Player::new();
                        current_block = 0;
                        next_block = get_next_block(max_blocks);
                        horizontal_shift = 0.0;
                    }
                }
            }
        }

        match event {
            Event::MainEventsCleared => {
                match &state {
                    State::SPLASH => {
                        draw_image(pixels.get_frame(), &splash_img);
                        if pixels.render().map_err(|e| {}).is_err() {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                    State::RUNNING => {
                        score += 1;
                        let update_ret =
                            player.update(&blocks, (current_block, next_block), horizontal_shift);
                        match update_ret {

                            Update::DEAD => {
                                state = State::GAMEOVER;

                                let gameover_file = File::open("gameover.wav").unwrap();
                                let source = Decoder::new(gameover_file).unwrap();

                                sink.append(source);
                                std::thread::sleep(Duration::from_millis(300));
                                draw_image(pixels.get_frame(), &gameover_img);
                                if pixels.render().map_err(|e| {}).is_err() {
                                    *control_flow = ControlFlow::Exit;
                                    return;
                                }
                                // std::thread::sleep(Duration::from_millis(1000));
                                save_highscore(std::cmp::max(score, highscore));

                                player = Player::new();
                                current_block = 9;
                                next_block = get_next_block(max_blocks);
                                horizontal_shift = 0.0;
                                return;
                            }
                            Update::NOTHING => {
                                let sound_duration = 2000 as u64;
                                if last_played.elapsed().as_millis() as u64 > sound_duration {
                                    let freq_mult = (player.pos_x - MIN_PLAYER_X as f32)
                                        / (MAX_PLAYER_X as f32 - MIN_PLAYER_X as f32);
                                    let source = SineWave::new(1000.0 * (1.0 - freq_mult).sqrt())
                                        .take_duration(Duration::from_millis(100))
                                        .amplify(0.20);
                                    sink.append(source);
                                    last_played = Instant::now();
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

                                draw_score_lives(
                                    score,
                                    highscore,
                                    player.get_lives(),
                                    &img,
                                    pixels.get_frame(),
                                );

                                let elapsed = last_update.elapsed();
                                let diff = frame_time - elapsed.as_millis() as i16;

                                if diff > 0 {
                                    // println!("sleeping for: {} ms", diff);
                                    thread::sleep(Duration::from_millis(diff as u64));
                                }

                                last_update = Instant::now();

                                if pixels.render().map_err(|e| {}).is_err() {
                                    *control_flow = ControlFlow::Exit;
                                    save_highscore(std::cmp::max(score, highscore));
                                    return;
                                }
                                horizontal_shift += 2.;
                                if horizontal_shift >= (HORIZONTAL_TILES * TILE_SCALE) as f32 {
                                    horizontal_shift = 0.0;
                                    current_block = next_block;
                                    // TODO: enable random blocks
                                    next_block = (current_block + 1) % max_blocks;
                                }
                            },

                            Update::SPEEDUP => {
                                if last_speedup_sound.elapsed().as_millis() > 500 {
                                    let file = File::open("speedup.wav").unwrap();
                                    let source = Decoder::new(file).unwrap();
                                    println!("Appending speedup");
                                    sink.append(source);
                                    last_speedup_sound = Instant::now();
                                }

                            }
                        }
                    }
                    State::GAMEOVER => {
                        draw_image(pixels.get_frame(), &gameover_img);
                        draw_score_lives(score, highscore, 0, &img, pixels.get_frame());
                        if pixels.render().map_err(|e| {}).is_err() {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                }
            }
            _ => {}
        }
    });
}

fn save_highscore(score: u64) {
    let mut file = File::create("highscore.txt").unwrap();
    writeln!(&mut file, "{}", score).unwrap();
}

fn get_highscore() -> u64 {
    match std::fs::read_to_string("highscore.txt") {
        Ok(s) => s.split_whitespace().collect::<Vec<_>>()[0]
            .parse::<u64>()
            .unwrap(),
        Err(_) => return 0,
    }
}

fn draw_score_lives(score: u64, orig_highscore: u64, lives: u8, img: &BitMap, pixels: &mut [u8]) {
    let numericals = (score as f64).log10() as u64 + 1;
    let end = 22 * TILE_SCALE as u64;

    for i in 0..numericals {
        let remainer = (score / 10u64.pow(i as u32)) % 10;
        let tile = img
            .crop(
                (100 + 10 * remainer) as u32,
                0,
                (100 + 10 * (remainer + 1)) as u32,
                10,
            )
            .unwrap();

        draw_tile(pixels, &tile, (end as i32 - 10 * i as i32, 21 as i32));
    }

    let highscore = std::cmp::max(score, orig_highscore);

    let numericals = (highscore as f64).log10() as u64 + 1;
    let end = 22 * TILE_SCALE as u64;

    for i in 0..numericals {
        let remainer = (highscore / 10u64.pow(i as u32)) % 10;
        let tile = img
            .crop(
                (200 + 10 * remainer) as u32,
                0,
                (200 + 10 * (remainer + 1)) as u32,
                10,
            )
            .unwrap();

        draw_tile(pixels, &tile, (end as i32 - 10 * i as i32, 10 as i32));
    }

    let heart = img.crop(40, 0, 50, 10).unwrap();
    for i in 0..lives {
        draw_tile(pixels, &heart, (end as i32 - 10 * i as i32, 32 as i32));
    }
}

fn load_blocks(num_maps: u32) -> Vec<BitMap> {
    let blocks = BitMap::read("blocks.bmp").unwrap();

    let mut result = vec![];
    for i in 0..num_maps {
        let mut block = blocks
            .crop(i * HORIZONTAL_TILES, 0, (i + 1) * HORIZONTAL_TILES, 16)
            .unwrap();
        let mut block_inverted = BitMap::new(48, 16);

        for y in 0..16 {
            for x in 0..HORIZONTAL_TILES {
                let pixel = block.get_pixel(x, 16 - y - 1).unwrap();
                block_inverted.set_pixel(x, y, *pixel);
            }
        }

        result.push(block_inverted);
    }

    result
}

struct Player {
    pos_x: f32,
    pos_y: f32,
    speed_y: f32,
    size_x: u32,
    size_y: u32,

    jump_info: JumpInfo,
    tile: BitMap,
    lives: u8,
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
            lives: 3,
        }
    }

    fn jump(&mut self) {
        if self.jump_info.on_ground {
            self.jump_info.jumping = true;
            self.jump_info.jump_start = Instant::now();
            self.speed_y -= 1.5;
        } else if self.jump_info.jumping {
            let elapsed = self.jump_info.jump_start.elapsed().as_millis();
            if elapsed > 100 && elapsed < 150 {
                self.speed_y -= 1.5;
                self.jump_info.jumping = false;
            }
        }
    }

    fn update(
        &mut self,
        blocks: &Vec<BitMap>,
        blocks_ids: (u32, u32),
        horizontal_shift: f32,
    ) -> Update {
        let steps = 5;
        let speed_y_fraction = self.speed_y / steps as f32;

        self.pos_x -= 0.01;

        if self.pos_x <= 0.0 {
            return Update::NOTHING;
        }
        if self.pos_y as u32 + self.size_y as u32 >= HEIGHT - 6 {
            return Update::DEAD;
        }

        // if self.pos_x < MIN_PLAYER_X as f32 {
            // self.pos_y += 3.0;
            // self.pos_x -= 3.0;
            // return Update::NOTHING;
        // }
        //

        for step in 0..steps {
            self.pos_y += speed_y_fraction;

            // Vertical
            let corners = get_corners(
                (self.pos_x as u32, self.pos_y as u32),
                (self.size_x, self.size_y),
            );
            let pixels = get_pixels(&corners, blocks, blocks_ids, horizontal_shift as u32);

            if pixels.iter().any(|p| is_solid(&p)) {
                self.pos_y -= speed_y_fraction;
                self.speed_y = 0.0;
                self.jump_info.on_ground = true;
                self.jump_info.jumping = false;

                if pixels.iter().any(|p| same_rgb(&p, &Rgb::new(99, 155, 255))) {
                    self.pos_x += 5.0;
                    if self.pos_x > MAX_PLAYER_X as f32 {
                        self.pos_x = MAX_PLAYER_X as f32;
                    }
                    return Update::SPEEDUP;
                } else if pixels.iter().any(|p| same_rgb(&p, &Rgb::new(217, 87, 99))) {
                    return Update::DEAD;
                }
                break;
            } else {
                self.jump_info.on_ground = false;
                self.speed_y += 0.15 / steps as f32;
            }
        }

        Update::NOTHING
    }

    fn draw(&self, pixels: &mut [u8]) {
        draw_tile(pixels, &self.tile, (self.pos_x as i32, self.pos_y as i32));
    }

    fn get_lives(&self) -> u8 {
        ((self.pos_x as u32 - MIN_PLAYER_X) / 3 + 1) as u8
    }
}

fn get_next_block(max_blocks: u32) -> u32 {
    0
}

fn get_pixels(
    positions: &Vec<(u32, u32)>,
    blocks: &Vec<BitMap>,
    blocks_ids: (u32, u32),
    horizontal_shift: u32,
) -> Vec<Rgb> {
    let mut results = vec![];
    for coords in positions {
        results.push(get_pixel(*coords, blocks, blocks_ids, horizontal_shift));
    }

    results
}

fn get_corners(coords: (u32, u32), size: (u32, u32)) -> Vec<(u32, u32)> {
    let (x, y) = coords;
    let (width, height) = size;

    let top_left = (x, y);
    let top_right = (x + width, y);
    let bottom_left = (x, y + height);
    let bottom_right = (x + width, y + height);

    vec![top_left, top_right, bottom_left, bottom_right]
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

fn draw_image(pixels: &mut [u8], image: &BitMap) {

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let index = y * WIDTH + x;
            let pixel = image.get_pixel(x, y).unwrap();
            pixels[4 * index as usize + 0] = pixel.get_red();
            pixels[4 * index as usize + 1] = pixel.get_green();
            pixels[4 * index as usize + 2] = pixel.get_blue();
            pixels[4 * index as usize + 3] = 255;
        }
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
