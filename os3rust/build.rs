use std::process::Command;

fn main() {
    println!(r"cargo:rustc-link-search=native=/opt/homebrew/lib");

    if std::env::var("TARGET") == Ok("wasm32-unknown-emscripten".to_string()) {
        Command::new("emsdk").args(["install", "4.0.10"]).output().unwrap();
        Command::new("emsdk").args(["activate", "4.0.10"]).output().unwrap();
        println!(r"cargo:rustc-link-arg=-sUSE_SDL=2");
        println!(r"cargo:rustc-link-arg=-sUSE_SDL_TTF=2");
        println!(r"cargo:rustc-link-arg=-sUSE_SDL_IMAGE=2");
        println!(r"cargo:rustc-link-arg=-sUSE_SDL_GFX=2");
        println!(r"cargo:rustc-link-arg=-sUSE_SDL_MIXER=2");
        // println!(r"cargo:rustc-link-arg=-sASYNCIFY");
    }

    return ();
}
