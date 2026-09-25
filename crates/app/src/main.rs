//! Window, main loop and platform glue. See ADR-0004.

mod keys;

use macroquad::prelude::*;
use trpg_ui::input::{InputState, Keymap};

fn window_conf() -> Conf {
    Conf {
        window_title: "tactical-rpg".to_owned(),
        window_width: 1600,
        window_height: 1024,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let content = match trpg_content::load_embedded() {
        Ok(content) => content,
        Err(errors) => return show_content_errors(&errors.to_string()).await,
    };
    let mut input = InputState::new(Keymap::from_def(&content.keymap));
    loop {
        let actions = keys::poll(&mut input);
        if cfg!(debug_assertions) {
            for action in &actions {
                info!("action: {}", action);
            }
        }
        clear_background(BLACK);
        draw_text("tactical-rpg", 20.0, 40.0, 30.0, WHITE);
        next_frame().await;
    }
}

/// Shows asset errors instead of the game. Only reachable if a broken asset
/// slipped past the content tests.
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
