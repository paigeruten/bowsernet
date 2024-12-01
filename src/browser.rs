use crate::{config::Dimensions, lex, request, ConnectionPool, RequestCache, Url};
use macroquad::prelude::*;

const HSTEP: i32 = 15;
const VSTEP: i32 = 18;
const SCROLL_STEP: i32 = 100;

pub struct Browser {
    connection_pool: ConnectionPool,
    request_cache: RequestCache,
    font: Font,
    display_text: String,
    display_list: Vec<DisplayItem>,
    dimensions: Dimensions,
    scroll: i32,
}

impl Browser {
    pub fn new() -> color_eyre::Result<Self> {
        Ok(Self {
            connection_pool: ConnectionPool::new(),
            request_cache: RequestCache::new(),
            font: load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Times New Roman.ttf"))?,
            display_text: "".to_string(),
            display_list: Vec::new(),
            dimensions: Dimensions {
                width: screen_width() as i32,
                height: screen_height() as i32,
            },
            scroll: 0,
        })
    }

    pub fn load(&mut self, url: &Url) -> color_eyre::Result<()> {
        let body = request(url, &mut self.connection_pool, &mut self.request_cache)?;
        self.display_text = lex(&body);
        self.display_list = layout(&self.display_text, self.dimensions.width);
        Ok(())
    }

    pub fn draw(&self) {
        for &DisplayItem { x, y, c } in self.display_list.iter() {
            if y > self.scroll + self.dimensions.height || y + VSTEP < self.scroll {
                continue;
            }

            draw_text_ex(
                &c.to_string(),
                x as f32,
                (y - self.scroll) as f32,
                TextParams {
                    font: Some(&self.font),
                    font_size: 20,
                    color: BLACK,
                    ..Default::default()
                },
            );
        }
    }

    pub fn handle_input(&mut self) {
        self.handle_resize();

        let (_, mouse_wheel_y) = mouse_wheel();
        self.scroll -= mouse_wheel_y as i32;

        if is_key_pressed(KeyCode::Space) {
            self.scroll += SCROLL_STEP;
        } else if is_key_down(KeyCode::Down) {
            self.scroll += 2;
        } else if is_key_down(KeyCode::Up) {
            self.scroll -= 2;
        }

        if self.scroll < 0 {
            self.scroll = 0;
        }
    }

    pub fn handle_resize(&mut self) {
        let new_dimensions = Dimensions {
            width: screen_width() as i32,
            height: screen_height() as i32,
        };
        if new_dimensions != self.dimensions {
            self.dimensions = new_dimensions;
            self.display_list = layout(&self.display_text, self.dimensions.width);
        }
    }
}

struct DisplayItem {
    pub x: i32,
    pub y: i32,
    pub c: char,
}

fn layout(text: &str, width: i32) -> Vec<DisplayItem> {
    let mut display_list = Vec::new();
    let mut cursor_x = HSTEP;
    let mut cursor_y = VSTEP;
    for c in text.chars() {
        if c == '\n' {
            cursor_x = HSTEP;
            cursor_y += VSTEP + (VSTEP / 2);
            continue;
        }
        display_list.push(DisplayItem {
            x: cursor_x,
            y: cursor_y,
            c,
        });
        cursor_x += HSTEP;
        if cursor_x >= width - HSTEP {
            cursor_y += VSTEP;
            cursor_x = HSTEP;
        }
    }
    display_list
}
