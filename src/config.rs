pub const DEFAULT_WIDTH: i32 = 800;
pub const DEFAULT_HEIGHT: i32 = 800;
pub const DEFAULT_URL: &str = "file://examples/welcome.html";

pub const SCROLL_BAR_WIDTH: f32 = 10.;

pub const FPS_WIDTH: f32 = 68.;
pub const FPS_HEIGHT: f32 = 16.;
pub const FPS_HPADDING: f32 = 2.;
pub const FPS_VPADDING: f32 = 12.;
pub const FPS_FONT_SIZE: f32 = 18.;

#[derive(Debug, PartialEq)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}
