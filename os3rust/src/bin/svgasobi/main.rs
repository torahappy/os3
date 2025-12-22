// svgで無双したいけどなあ...

extern crate sdl2;

use askama::Template;

#[cfg(not(all(target_arch = "wasm32", target_os = "emscripten")))]
use quanta::Clock;

use resvg::render;
use resvg::tiny_skia;
use resvg::tiny_skia::PixmapMut;
use resvg::usvg::fontdb::Database;
use resvg::usvg::{Transform, Tree};
use rust_embed::Embed;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::sys::SDL_GetWindowSizeInPixels;
use sdl2::video::Window;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "example.svg")]
struct HelloTemplate<'a> {
    my_x: &'a f32,
    elapsed: &'a f64,
    delta: &'a f64,
}

#[derive(Embed)]
#[folder = "data/"]
struct Asset;

fn get_window_size_in_pixels(win: &Window) -> (i32, i32) {
    let mut width: i32 = 0;
    let mut height: i32 = 0;

    unsafe {
        SDL_GetWindowSizeInPixels(win.raw(), &mut width, &mut height);
    }
    return (width, height);
}

#[cfg(all(target_arch = "wasm32", target_os = "emscripten"))]
unsafe extern "C" {
    unsafe fn emscripten_sleep(x: u32);
}

#[cfg(all(target_arch = "wasm32", target_os = "emscripten"))]
fn emscripten_sleep_zero() {
    unsafe {
        emscripten_sleep(0);
    }
}

fn run() -> Result<(), String> {
    let mut SCREEN_WIDTH: u32 = 1000;
    let mut SCREEN_HEIGHT: u32 = 1000;

    println!("load font");
    let fdb = {
        let mut f = Database::new();
        f.load_font_data(
            Asset::get("NotoSerifJP-VariableFont_wght.ttf")
                .unwrap()
                .data
                .to_vec(),
        );
        f.load_font_data(
            Asset::get("ZenKakuGothicAntique-Regular.ttf")
                .unwrap()
                .data
                .to_vec(),
        );
        f
    };

    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;

    let window = video_subsys
        .window("ぐるぐるテスト", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .resizable()
        .opengl()
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;

    let density = {
        let (raw_w, raw_h) = get_window_size_in_pixels(&window);

        println!("Raw Window Size: {} {}", raw_w, raw_h);

        let density: f32 = (raw_w as f32) / (SCREEN_WIDTH as f32);

        println!("Density: {}", density);

        density
    };

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
    canvas.clear();

    let mut texture2 = texture_creator
        .create_texture_streaming(PixelFormatEnum::ABGR8888, SCREEN_WIDTH, SCREEN_HEIGHT)
        .unwrap();

    let mut frame: u64 = 0;
    let mut elapsed: f64 = 0.0;

    #[cfg(not(all(target_arch = "wasm32", target_os = "emscripten")))]
    let (timer, mut time_a, mut time_b) = {
        let timer = Clock::new();
        let time_a = timer.now();
        let time_b = timer.now();
        (timer, time_a, time_b)
    };

    let opt = resvg::usvg::Options {
        resources_dir: None,
        dpi: 72.0 * density,
        font_family: "Noto Serif JP".to_string(),
        font_size: 16.0,
        languages: vec!["en".to_string()],
        fontdb: Arc::new(fdb.clone()),
        text_rendering: resvg::usvg::TextRendering::OptimizeSpeed,
        shape_rendering: resvg::usvg::ShapeRendering::OptimizeSpeed,
        image_rendering: resvg::usvg::ImageRendering::OptimizeSpeed,
        ..Default::default()
    };

    'mainloop: loop {
        #[cfg(not(all(target_arch = "wasm32", target_os = "emscripten")))]
        let delta = {
            time_b = timer.now();
            let delta = time_b.duration_since(time_a).as_secs_f64();
            time_a = timer.now();
            delta
        };

        #[cfg(all(target_arch = "wasm32", target_os = "emscripten"))]
        let delta: f64 = 1.0 / 60.0;

        let svg_data = (HelloTemplate {
            my_x: &((elapsed * 10.0) as f32),
            elapsed: &elapsed,
            delta: &delta,
        })
        .render()
        .unwrap();
        let t = Tree::from_str(&svg_data, &opt).unwrap();

        frame += 1;
        elapsed += delta;
        let raw_w = (density * (SCREEN_WIDTH as f32)) as u32;
        let raw_h = (density * (SCREEN_HEIGHT as f32)) as u32;
        let mut binding = vec![0; (raw_w as usize) * (raw_h as usize) * 4];
        let mut p = PixmapMut::from_bytes(binding.as_mut_slice(), raw_w, raw_h).unwrap();

        p.fill(tiny_skia::Color::from_rgba8(255, 255, 255, 255));

        render(
            &t,
            Transform {
                sx: 1.0,
                kx: 0.0,
                ky: 0.0,
                sy: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            &mut p,
        );

        texture2
            .update(None, &binding.as_slice(), (raw_w as usize) * 4)
            .unwrap();
        canvas.copy(&texture2, None, None)?;
        canvas.present();

        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        #[cfg(all(target_arch = "wasm32", target_os = "emscripten"))]
        emscripten_sleep_zero();
    }

    Ok(())
}

fn main() -> Result<(), String> {
    run()?;
    Ok(())
}
