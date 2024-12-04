use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::read_to_string,
};

use crate::{
    config::{Dimensions, SCROLL_BAR_WIDTH},
    html::Token,
    lex, request, ConnectionPool, RequestCache, Url,
};
use macroquad::prelude::*;

const PADDING: i32 = 24;
const SCROLL_STEP: i32 = 100;
const FONT_SIZE: u16 = 20;
const SUPPORTED_EMOJIS: &str = "\
    \u{1F600}\u{1F60A}\u{1F60B}\u{1F60D}\u{1F61A}\u{1F61B}\u{1F61C}\u{1F61D}\
    \u{1F92A}\u{1F600}\u{1F601}\u{1F602}\u{1F603}\u{1F604}\u{1F605}\u{1F606}\
    \u{1F607}\u{1F609}\u{1F617}\u{1F618}\u{1F619}\u{1F642}\u{1F643}\u{1F923}\
    \u{1F929}\u{1F970}\u{1F972}\u{1FAE0}\u{263A}";

pub struct Browser {
    connection_pool: ConnectionPool,
    request_cache: RequestCache,
    font_group: FontGroup,
    emoji_cache: HashMap<char, Texture2D>,
    supported_emojis: HashSet<char>,
    display_tokens: Vec<Token>,
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
            font_group: FontGroup {
                normal: load_ttf_font_from_bytes(include_bytes!(
                    "../assets/fonts/Times New Roman.ttf"
                ))?,
                italic: load_ttf_font_from_bytes(include_bytes!(
                    "../assets/fonts/Times New Roman Italic.ttf"
                ))?,
                bold: load_ttf_font_from_bytes(include_bytes!(
                    "../assets/fonts/Times New Roman Bold.ttf"
                ))?,
                bold_italic: load_ttf_font_from_bytes(include_bytes!(
                    "../assets/fonts/Times New Roman Bold Italic.ttf"
                ))?,
            },
            emoji_cache: HashMap::new(),
            supported_emojis: HashSet::from_iter(SUPPORTED_EMOJIS.chars()),
            display_tokens: Vec::new(),
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
        self.display_tokens = lex(&body);
        self.reflow();
        Ok(())
    }

    fn reflow(&mut self) {
        self.display_list = layout(
            &self.display_tokens,
            self.dimensions.width,
            &self.font_group,
        );
        self.scroll_max = self
            .display_list
            .iter()
            .map(|disp| disp.y as i32)
            .max()
            .unwrap_or(0);
        self.scroll_max = (self.scroll_max - self.dimensions.height + PADDING).max(0);
        if self.scroll > self.scroll_max {
            self.scroll = self.scroll_max;
        }
    }

    pub fn draw(&mut self) {
        for DisplayItem { x, y, word, style } in self.display_list.iter() {
            if *y as i32 > self.scroll + self.dimensions.height || *y as i32 + PADDING < self.scroll
            {
                continue;
            }

            let first_char = word.chars().next();
            if first_char.is_some() && self.supported_emojis.contains(&first_char.unwrap()) {
                let emoji_texture =
                    self.emoji_cache
                        .entry(first_char.unwrap())
                        .or_insert_with(|| {
                            let svg = read_to_string(
                                File::open(format!(
                                    "assets/emoji/{:X}.svg",
                                    first_char.unwrap() as u32
                                ))
                                .unwrap(),
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
                    x - 18.,
                    (y - self.scroll as f32) - 24.,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(36., 36.)),
                        ..Default::default()
                    },
                );
            } else {
                draw_text_ex(
                    word,
                    *x,
                    y - self.scroll as f32,
                    TextParams {
                        font: Some(self.font_group.get(*style)),
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

#[derive(Debug, Clone, Copy)]
enum FontStyle {
    Normal,
    Bold,
    Italic,
    BoldItalic,
}

struct FontGroup {
    pub normal: Font,
    pub italic: Font,
    pub bold: Font,
    pub bold_italic: Font,
}

impl FontGroup {
    pub fn get(&self, style: FontStyle) -> &Font {
        match style {
            FontStyle::Normal => &self.normal,
            FontStyle::Bold => &self.bold,
            FontStyle::Italic => &self.italic,
            FontStyle::BoldItalic => &self.bold_italic,
        }
    }
}

struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub word: String,
    pub style: FontStyle,
}

fn layout(tokens: &[Token], screen_width: i32, font_group: &FontGroup) -> Vec<DisplayItem> {
    let mut display_list = Vec::new();
    let mut cursor_x = PADDING as f32;
    let mut cursor_y = PADDING as f32;
    let mut bold = false;
    let mut italic = false;
    let space_width = measure_text(" ", Some(&font_group.normal), FONT_SIZE, 1.).width;
    let line_height = measure_text("X", Some(&font_group.normal), FONT_SIZE, 1.).height * 1.75;
    for token in tokens {
        match token {
            Token::Text(text) => {
                let style = match (bold, italic) {
                    (false, false) => FontStyle::Normal,
                    (false, true) => FontStyle::Italic,
                    (true, false) => FontStyle::Bold,
                    (true, true) => FontStyle::BoldItalic,
                };
                let font = font_group.get(style);
                for word in text.split_whitespace() {
                    display_list.push(DisplayItem {
                        x: cursor_x,
                        y: cursor_y,
                        word: word.to_string(),
                        style,
                    });
                    let dimensions = measure_text(word, Some(font), FONT_SIZE, 1.);
                    cursor_x += dimensions.width + space_width;
                    if cursor_x >= (screen_width - PADDING) as f32 {
                        cursor_y += line_height;
                        cursor_x = PADDING as f32;
                    }
                }
            }
            Token::Tag(tag) => {
                if tag == "br" || tag == "br /" || tag == "br/" {
                    cursor_x = PADDING as f32;
                    cursor_y += line_height;
                } else if tag == "/p" {
                    cursor_x = PADDING as f32;
                    cursor_y += line_height * 2.;
                } else if tag == "i" || tag == "em" {
                    italic = true;
                } else if tag == "/i" || tag == "/em" {
                    italic = false;
                } else if tag == "b" || tag == "strong" {
                    bold = true;
                } else if tag == "/b" || tag == "/strong" {
                    bold = false;
                }
            }
        }
    }
    display_list
}
