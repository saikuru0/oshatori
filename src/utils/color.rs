use kanii_lib::packets::types::Color;

pub fn kanii_to_rgba(color: Color) -> Option<[u8; 4]> {
    if let Ok(rgba) = color.as_rgba() {
        Some(rgba)
    } else {
        None
    }
}
