extern crate sdl2;

use resvg::render;
use resvg::tiny_skia;
use resvg::tiny_skia::{IntSize, PixmapMut};
use resvg::usvg::fontdb::Database;
use resvg::usvg::{FontResolver, ImageHrefResolver, Options, Size, Transform, Tree};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormat, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use std::fs;
use std::path::Path;
use std::sync::Arc;

static SCREEN_WIDTH: u32 = 800;
static SCREEN_HEIGHT: u32 = 600;

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

// Scale fonts to a reasonable size when they're too big (though they might look less smooth)
fn get_centered_rect(rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
    let wr = rect_width as f32 / cons_width as f32;
    let hr = rect_height as f32 / cons_height as f32;

    let (w, h) = if wr > 1f32 || hr > 1f32 {
        if wr > hr {
            println!("Scaling down! The text will look worse!");
            let h = (rect_height as f32 / wr) as i32;
            (cons_width as i32, h)
        } else {
            println!("Scaling down! The text will look worse!");
            let w = (rect_width as f32 / hr) as i32;
            (w, cons_height as i32)
        }
    } else {
        (rect_width as i32, rect_height as i32)
    };

    let cx = (SCREEN_WIDTH as i32 - w) / 2;
    let cy = (SCREEN_HEIGHT as i32 - h) / 2;
    rect!(cx, cy, w, h)
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

fn run(font_path: &Path) -> Result<(), String> {
    let mut fdb = Database::new();
    fdb.load_system_fonts();

    let t = Tree::from_data(
        fs::read("example.svg").unwrap().as_slice(),
        &resvg::usvg::Options {
            resources_dir: None,
            dpi: 200.0,
            font_family: "'Noto Serif JP', serif".to_string(),
            font_size: 16.0,
            languages: vec!["en".to_string()],
            fontdb: Arc::new(fdb),
            ..Default::default()
        },
    )
    .unwrap();

    let mut binding = vec![0; 1000 * 1000 * 4];
        let mut p = PixmapMut::from_bytes(binding.as_mut_slice(), 1000, 1000).unwrap();

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

    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsys
        .window("SDL2_TTF Example", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    // Load a font
    let mut font = ttf_context.load_font(font_path, 60)?;
    font.set_style(sdl2::ttf::FontStyle::NORMAL);

    // render a surface, and convert it to a texture bound to the canvas
    //let surface = font
    //    .render("あいうえお")
    //    .blended(Color::RGBA(255, 0, 0, 255))
    //    .map_err(|e| e.to_string())?;
    //let texture = texture_creator
    //    .create_texture_from_surface(&surface)
    //    .map_err(|e| e.to_string())?;

    let mut texture2 = texture_creator
        .create_texture_streaming(PixelFormatEnum::ABGR8888, 1000, 1000)
        .unwrap();
    texture2.update(None, &binding.as_slice(), 1000*4).unwrap();

    canvas.set_draw_color(Color::RGBA(195, 217, 255, 255));
    canvas.clear();

    // let TextureQuery { width, height, .. } = texture.query();

    'mainloop: loop {
    // If the example text is too big for the screen, downscale it (and center irregardless)
    let padding = 64;
    //let target = get_centered_rect(
    //    width,
    //    height,
    //    SCREEN_WIDTH - padding,
    //    SCREEN_HEIGHT - padding,
    //);

    //canvas.copy(&texture, None, Some(target))?;
    //canvas.present();
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
    let path: &Path = Path::new("./ZenKakuGothicAntique-Regular.ttf");
    run(path)?;
    Ok(())
}
