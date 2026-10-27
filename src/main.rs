//use std::env::Args;

use std::ops::Div;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use argh::FromArgs;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{ScaleMode, TextureAccess};
use sdl3::sys::everything::SDL_RendererLogicalPresentation;

pub mod chip8;

use chip8::Chip8;
use crate::chip8::{GFX_HEIGHT, GFX_WIDTH};

/// Chip8
#[derive(FromArgs)]
struct Args {
    /// rom
    #[argh(positional)]
    rom_file: PathBuf
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();
    let file = std::fs::read(args.rom_file)?;
    let mut chip8: Chip8 = Chip8::new(file.as_slice()).map_err(|msg| { anyhow::anyhow!(msg) })?;

    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;
    let audio_subsystem = sdl_context.audio()?;

    let scale: u32 = 25;
    let window = video_subsystem.window("ficha8", (GFX_WIDTH as u32) * scale, (GFX_HEIGHT as u32) * scale)
        .position_centered()
        .resizable()
        .build()?;

    let mut canvas = window.into_canvas();
    let mut event_pump = sdl_context.event_pump()?;
    let mut texture = canvas.create_texture(PixelFormat::ARGB8888, TextureAccess::Streaming, GFX_WIDTH as u32, GFX_HEIGHT as u32)?;
    texture.set_scale_mode(ScaleMode::Nearest);

    unsafe {
        sdl3::sys::render::SDL_SetRenderLogicalPresentation(canvas.raw(), GFX_WIDTH as i32, GFX_HEIGHT as i32, SDL_RendererLogicalPresentation::LETTERBOX);
    }

    'running: loop {

        let start: Instant = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} => {
                    break 'running;
                }
                Event::KeyDown { keycode: Some(keycode) , ..} => {
                    poll_chip8_keypad(&mut chip8, keycode, true);
                }
                Event::KeyUp { keycode: Some(keycode) , ..} => {
                    poll_chip8_keypad(&mut chip8, keycode, false);
                }
                _ => {}
            }
        }

        chip8.execute_frame().map_err(|msg| { anyhow::anyhow!(msg) })?;

        texture.update(None, bytemuck::cast_slice(&chip8.dump_argb8()), GFX_WIDTH * size_of::<u32>())?;
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();

        let duration = Duration::from_secs(1).div(chip8::FPS);
        std::thread::sleep(duration.saturating_sub(Instant::now() - start));
    }

    Ok(())
}

fn poll_chip8_keypad(chip8: &mut Chip8, keycode: Keycode, value: bool) {
    match keycode {
        Keycode::_1 => chip8.set_key(0x1, value),
        Keycode::_2 => chip8.set_key(0x2, value),
        Keycode::_3 => chip8.set_key(0x3, value),
        Keycode::_4 => chip8.set_key(0xC, value),
        Keycode::Q => chip8.set_key(0x4, value),
        Keycode::W => chip8.set_key(0x5, value),
        Keycode::E => chip8.set_key(0x6, value),
        Keycode::R => chip8.set_key(0xD, value),
        Keycode::A => chip8.set_key(0x7, value),
        Keycode::S => chip8.set_key(0x8, value),
        Keycode::D => chip8.set_key(0x9, value),
        Keycode::F => chip8.set_key(0xE, value),
        Keycode::Z => chip8.set_key(0xA, value),
        Keycode::X => chip8.set_key(0x0, value),
        Keycode::C => chip8.set_key(0xB, value),
        Keycode::V => chip8.set_key(0xF, value),
        _ => {}
    }
}
