//! Window, main loop and platform glue. See ADR-0004.

mod keys;
mod render;

use macroquad::prelude::*;
use trpg_content::font::ATLAS_PNG_PATH;
use trpg_ui::{Ctx, Game, UiColor};

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
    let ctx = match Ctx::embedded() {
        Ok(ctx) => ctx,
        Err(e) => return show_content_errors(&e.to_string()).await,
    };
    let png = trpg_content::bundle::bytes(ATLAS_PNG_PATH).unwrap_or_default();
    let black = ctx.palette.get(UiColor::Black);
    let mut renderer = match Renderer::new(ctx.content.font.clone(), png, black) {
        Ok(renderer) => renderer,
        Err(e) => return show_content_errors(&e).await,
    };
    let mut game = Game::start(ctx);
    let mut running = true;
    loop {
        let events = keys::poll();
        if running {
            let out = game.frame(&events, get_frame_time());
            renderer.draw(out.buffer);
            if out.quit {
                // Native: leaving main closes the window. The web page can't
                // be closed, so it stops updating and keeps the last frame.
                if cfg!(target_arch = "wasm32") {
                    running = false;
                } else {
                    break;
                }
            }
        } else {
            renderer.draw(game.buffer());
        }
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
