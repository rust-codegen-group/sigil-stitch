#[derive(Debug)]
pub enum Message {
    Quit,
    Move {
        x: i32,
        y: i32,
    },
    Write(String),
    ChangeColor {
        r: u8,
        g: u8,
        b: u8,
    },
}
