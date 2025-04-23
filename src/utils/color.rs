use kanii_lib::packets::types::Color;

pub fn kanii_to_rgba(color: Color) -> Option<[u8; 4]> {
    if let Ok(rgb) = color.as_rgb() {
        let r: u8 = rgb.0 as u8;
        let g: u8 = rgb.1 as u8;
        let b: u8 = rgb.2 as u8;
        Some([r, g, b, 0])
    } else {
        None
    }
}
