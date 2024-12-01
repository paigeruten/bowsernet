use macroquad::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use bowsernet::{
    config::{DEFAULT_HEIGHT, DEFAULT_URL, DEFAULT_WIDTH},
    Browser, Url,
};

fn window_conf() -> Conf {
    Conf {
        window_title: "bowsernet".to_owned(),
        window_width: DEFAULT_WIDTH,
        window_height: DEFAULT_HEIGHT,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    setup_tracing()?;

    let args: Vec<String> = std::env::args().collect();

    let mut browser = Browser::new()?;

    let url = Url::parse(args.get(1).unwrap_or(&DEFAULT_URL.to_string()))?;
    browser.load(&url)?;

    let mut frame: u64 = 0;
    let mut fps = format!("FPS: {}", get_fps());
    loop {
        clear_background(WHITE);

        browser.handle_input();
        browser.draw();

        if frame % 20 == 0 {
            fps = format!("FPS: {}", get_fps());
        }
        draw_rectangle(
            screen_width() - 68.,
            0.,
            68.,
            16.,
            Color { a: 0.6, ..BLACK },
        );
        draw_text(&fps, screen_width() - 66., 12., 18., WHITE);

        next_frame().await;
        frame += 1;
    }
}

pub fn setup_tracing() -> color_eyre::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = tracing_subscriber::fmt::format().pretty();
    let formatting_layer = tracing_subscriber::fmt::layer().event_format(format);
    tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .init();
    Ok(())
}
