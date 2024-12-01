pub const DEFAULT_WIDTH: i32 = 800;
pub const DEFAULT_HEIGHT: i32 = 800;
pub const DEFAULT_URL: &str = "file://examples/welcome.html";

#[derive(Debug, PartialEq)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}
