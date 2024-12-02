use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::read_to_string,
};

use crate::{
    config::{Dimensions, SCROLL_BAR_WIDTH},
    lex, request, ConnectionPool, RequestCache, Url,
};
use macroquad::prelude::*;

const HSTEP: i32 = 15;
const VSTEP: i32 = 18;
const SCROLL_STEP: i32 = 100;
const SUPPORTED_EMOJIS: &str = "\
    \u{1F600}\u{1F60A}\u{1F60B}\u{1F60D}\u{1F61A}\u{1F61B}\u{1F61C}\u{1F61D}\
    \u{1F92A}\u{1F600}\u{1F601}\u{1F602}\u{1F603}\u{1F604}\u{1F605}\u{1F606}\
    \u{1F607}\u{1F609}\u{1F617}\u{1F618}\u{1F619}\u{1F642}\u{1F643}\u{1F923}\
    \u{1F929}\u{1F970}\u{1F972}\u{1FAE0}\u{263A}";

pub struct Browser {
    connection_pool: ConnectionPool,
    request_cache: RequestCache,
    font: Font,
    emoji_cache: HashMap<char, Texture2D>,
    supported_emojis: HashSet<char>,
    display_text: String,
    display_list: Vec<DisplayItem>,
    dimensions: Dimensions,
    scroll: i32,
    scroll_max: i32,
}

impl Browser {
    pub fn new() -> color_eyre::Result<Self> {
        Ok(Self {
            connection_pool: ConnectionPool::new(),
            request_cache: RequestCache::new(),
            font: load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Times New Roman.ttf"))?,
            emoji_cache: HashMap::new(),
            supported_emojis: HashSet::from_iter(SUPPORTED_EMOJIS.chars()),
            display_text: "".to_string(),
            display_list: Vec::new(),
            dimensions: Dimensions {
                width: screen_width() as i32,
                height: screen_height() as i32,
            },
            scroll: 0,
            scroll_max: 0,
        })
    }

    pub fn load(&mut self, url: &Url) -> color_eyre::Result<()> {
        let body = request(url, &mut self.connection_pool, &mut self.request_cache)?;
        self.display_text = lex(&body);
        self.reflow();
        Ok(())
    }

    fn reflow(&mut self) {
        self.display_list = layout(&self.display_text, self.dimensions.width);
        self.scroll_max = self
            .display_list
            .iter()
            .filter(|disp| !disp.c.is_whitespace())
            .map(|disp| disp.y)
            .max()
            .unwrap_or(0);
        self.scroll_max = (self.scroll_max - self.dimensions.height + VSTEP).max(0);
        if self.scroll > self.scroll_max {
            self.scroll = self.scroll_max;
        }
    }

    pub fn draw(&mut self) {
        for &DisplayItem { x, y, c } in self.display_list.iter() {
            if y > self.scroll + self.dimensions.height || y + VSTEP < self.scroll {
                continue;
            }

            if self.supported_emojis.contains(&c) {
                let emoji_texture = self.emoji_cache.entry(c).or_insert_with(|| {
                    let svg = read_to_string(
                        File::open(format!("assets/emoji/{:X}.svg", c as u32)).unwrap(),
                    )
                    .unwrap();
                    let tree = resvg::usvg::Tree::from_str(
                        &svg,
                        &resvg::usvg::Options {
                            dpi: 96. * 4.,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    let mut pixmap = resvg::tiny_skia::Pixmap::new(72, 72).unwrap();
                    resvg::render(&tree, Default::default(), &mut pixmap.as_mut());
                    let png_data = pixmap.encode_png().unwrap();
                    Texture2D::from_file_with_format(&png_data, Some(ImageFormat::Png))
                });

                draw_texture_ex(
                    emoji_texture,
                    x as f32 - 18.,
                    (y - self.scroll) as f32 - 24.,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(36., 36.)),
                        ..Default::default()
                    },
                );
            } else {
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

        if self.scroll_max > 0 {
            let scroll_screens = 1. + self.scroll_max as f32 / self.dimensions.height as f32;
            let scroll_bar_height = (self.dimensions.height as f32 / scroll_screens).max(10.);
            draw_rectangle(
                self.dimensions.width as f32 - SCROLL_BAR_WIDTH,
                self.scroll as f32 / self.scroll_max as f32
                    * (self.dimensions.height as f32 - scroll_bar_height),
                SCROLL_BAR_WIDTH,
                scroll_bar_height,
                Color { a: 0.6, ..BLUE },
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
        } else if self.scroll > self.scroll_max {
            self.scroll = self.scroll_max;
        }
    }

    fn handle_resize(&mut self) {
        let new_dimensions = Dimensions {
            width: screen_width() as i32,
            height: screen_height() as i32,
        };
        if new_dimensions != self.dimensions {
            self.dimensions = new_dimensions;
            self.reflow();
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
