use std::{
    cell::RefCell,
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
            font_group: FontGroup::new(),
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
        self.display_list = {
            let mut layout = Layout::new(self.dimensions.width, &self.font_group);
            layout.process_tokens(&self.display_tokens);
            layout.take_display_list()
        };
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
        for DisplayItem {
            x,
            y,
            word,
            style,
            font_size,
        } in self.display_list.iter()
        {
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
                        font_size: *font_size,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    measure_cache: RefCell<HashMap<(String, u16, FontStyle), TextDimensions>>,
}

impl FontGroup {
    pub fn new() -> Self {
        Self {
            normal: load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Times New Roman.ttf"))
                .unwrap(),
            italic: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/Times New Roman Italic.ttf"
            ))
            .unwrap(),
            bold: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/Times New Roman Bold.ttf"
            ))
            .unwrap(),
            bold_italic: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/Times New Roman Bold Italic.ttf"
            ))
            .unwrap(),
            measure_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn get(&self, style: FontStyle) -> &Font {
        match style {
            FontStyle::Normal => &self.normal,
            FontStyle::Bold => &self.bold,
            FontStyle::Italic => &self.italic,
            FontStyle::BoldItalic => &self.bold_italic,
        }
    }

    pub fn measure_text(&self, text: &str, font_size: u16, style: FontStyle) -> TextDimensions {
        *self
            .measure_cache
            .borrow_mut()
            .entry((text.to_string(), font_size, style))
            .or_insert_with(|| measure_text(text, Some(self.get(style)), font_size, 1.))
    }
}

#[derive(Debug, Clone)]
struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub word: String,
    pub style: FontStyle,
    pub font_size: u16,
}

struct Layout<'a> {
    display_list: Vec<DisplayItem>,
    line: Vec<DisplayItem>,
    font_group: &'a FontGroup,
    cursor_x: f32,
    cursor_y: f32,
    screen_width: i32,
    bold: bool,
    italic: bool,
    font_size: u16,
    in_head: bool,
}

impl<'a> Layout<'a> {
    pub fn new(screen_width: i32, font_group: &'a FontGroup) -> Self {
        Self {
            display_list: Vec::new(),
            line: Vec::new(),
            font_group,
            cursor_x: PADDING as f32,
            cursor_y: PADDING as f32,
            screen_width,
            bold: false,
            italic: false,
            font_size: FONT_SIZE,
            in_head: false,
        }
    }

    pub fn process_tokens(&mut self, tokens: &[Token]) {
        for token in tokens {
            match token {
                Token::Text(text) => {
                    if !self.in_head {
                        self.process_text(text);
                    }
                }
                Token::Tag(tag) => {
                    self.process_tag(tag);
                }
            }
        }
        self.flush_line();
    }

    fn process_text(&mut self, text: &str) {
        let style = match (self.bold, self.italic) {
            (false, false) => FontStyle::Normal,
            (false, true) => FontStyle::Italic,
            (true, false) => FontStyle::Bold,
            (true, true) => FontStyle::BoldItalic,
        };
        for word in text.split_whitespace() {
            self.line.push(DisplayItem {
                x: self.cursor_x,
                y: 0.,
                word: word.to_string(),
                style,
                font_size: self.font_size,
            });
            let text_width = self
                .font_group
                .measure_text(word, self.font_size, style)
                .width;
            self.cursor_x += text_width + self.space_width();
            if self.cursor_x >= (self.screen_width - PADDING) as f32 {
                self.flush_line();
            }
        }
    }

    fn process_tag(&mut self, tag: &str) {
        if tag == "head" {
            self.in_head = true;
        } else if tag == "/head" {
            self.in_head = false;
        } else if tag == "br" || tag == "br /" || tag == "br/" {
            self.flush_line();
        } else if tag == "/p" {
            self.flush_line();
            self.cursor_y += FONT_SIZE as f32;
        } else if tag == "i" || tag == "em" {
            self.italic = true;
        } else if tag == "/i" || tag == "/em" {
            self.italic = false;
        } else if tag == "b" || tag == "strong" {
            self.bold = true;
        } else if tag == "/b" || tag == "/strong" {
            self.bold = false;
        } else if tag == "small" {
            self.font_size -= 2;
        } else if tag == "/small" {
            self.font_size += 2;
        } else if tag == "big" {
            self.font_size += 4;
        } else if tag == "/big" {
            self.font_size -= 4;
        }
    }

    fn flush_line(&mut self) {
        if self.line.is_empty() {
            return;
        }

        let metrics: Vec<TextDimensions> = self
            .line
            .iter()
            .map(|item| {
                self.font_group
                    .measure_text(&item.word, item.font_size, item.style)
            })
            .collect();

        let max_ascent = metrics.iter().map(|dim| dim.offset_y as i32).max().unwrap() as f32;
        let baseline = self.cursor_y + 1.25 * max_ascent;

        for item in self.line.drain(..) {
            self.display_list.push(DisplayItem {
                y: baseline,
                ..item
            })
        }

        let max_descent = metrics
            .iter()
            .map(|dim| (dim.height - dim.offset_y) as i32)
            .max()
            .unwrap() as f32;

        self.cursor_y = baseline + 1.25 * max_descent;
        self.cursor_x = PADDING as f32;
    }

    fn space_width(&self) -> f32 {
        self.font_group
            .measure_text(" ", self.font_size, FontStyle::Normal)
            .width
    }

    pub fn take_display_list(self) -> Vec<DisplayItem> {
        self.display_list
    }
}
