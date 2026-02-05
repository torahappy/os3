// svgで無双したいけどなあ...

use askama::Template;

use quanta::Clock;

use resvg::render;
use resvg::tiny_skia;
use resvg::tiny_skia::PixmapMut;
use resvg::usvg::fontdb::Database;
use resvg::usvg::{Transform, Tree};
use rust_embed::Embed;
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

fn run() -> Result<(), String> {
    let mut SCREEN_WIDTH: u32 = 1000;
    let mut SCREEN_HEIGHT: u32 = 1000;
    let density = 1.0;

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

    let mut frame: u64 = 0;
    let mut elapsed: f64 = 0.0;

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
    }

    Ok(())
}

fn main() -> Result<(), String> {
    run()?;
    Ok(())
}
