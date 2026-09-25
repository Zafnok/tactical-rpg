//! Window, main loop and platform glue. See ADR-0004.

use macroquad::prelude::*;

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
    loop {
        clear_background(BLACK);
        draw_text("tactical-rpg", 20.0, 40.0, 30.0, WHITE);
        next_frame().await;
    }
}
