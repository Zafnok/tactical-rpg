//! Window, main loop and platform glue. See ADR-0004.

mod keys;
mod render;

use macroquad::prelude::*;
use trpg_content::font::ATLAS_PNG_PATH;
use trpg_ui::input::{InputState, Keymap};
use trpg_ui::{Palette, UiColor};

use crate::render::Renderer;

fn window_conf() -> Conf {
    Conf {
        window_title: "tactical-rpg".to_owned(),
        // The console at 2x (1600x1024) plus a small margin: the framebuffer
        // can come out a pixel smaller than asked, which would drop to 1x.
        window_width: 1640,
        window_height: 1064,
        window_resizable: true,
        // Full-resolution framebuffer on scaled displays, so the console's
        // integer scale is in real pixels (see render.rs).
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let content = match trpg_content::load_embedded() {
        Ok(content) => content,
        Err(errors) => return show_content_errors(&errors.to_string()).await,
    };
    let palette = match Palette::new(&content.palette) {
        Ok(palette) => palette,
        Err(missing) => {
            return show_content_errors(&format!("palette lacks {}", missing.join(", "))).await;
        }
    };
    let png = trpg_content::bundle::bytes(ATLAS_PNG_PATH).unwrap_or_default();
    let glyphs: Vec<char> = content.font.glyphs.keys().copied().collect();
    let mut renderer = match Renderer::new(content.font, png, palette.get(UiColor::Black)) {
        Ok(renderer) => renderer,
        Err(e) => return show_content_errors(&e).await,
    };
    // Screens arrive with ticket 0205; until then the app shows the sampler.
    let sampler = trpg_ui::debug::glyph_sampler(&palette, &glyphs);
    let mut input = InputState::new(Keymap::from_def(&content.keymap));
    loop {
        let actions = keys::poll(&mut input);
        if cfg!(debug_assertions) {
            for action in &actions {
                info!("action: {}", action);
            }
        }
        renderer.draw(&sampler);
        next_frame().await;
    }
}

/// Shows asset errors instead of the game (with macroquad's built-in font,
/// since ours may be what failed). Only reachable if a broken asset slipped
/// past the content tests.
async fn show_content_errors(text: &str) {
    error!("{}", text);
    loop {
        clear_background(BLACK);
        let mut y = 40.0;
        for line in text.lines() {
            draw_text(line, 20.0, y, 22.0, RED);
            y += 24.0;
        }
        next_frame().await;
    }
}
