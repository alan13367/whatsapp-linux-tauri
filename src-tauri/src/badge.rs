use image::{imageops::FilterType, Rgba, RgbaImage};

const ICON_SIZE: u32 = 64;
const BASE_ICON: &[u8] = include_bytes!("../icons/icon.png");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn render(unread: u32) -> Result<BadgeImage, image::ImageError> {
    let decoded = image::load_from_memory(BASE_ICON)?;
    let mut icon = decoded
        .resize_exact(ICON_SIZE, ICON_SIZE, FilterType::Lanczos3)
        .to_rgba8();

    if unread > 0 {
        draw_badge(&mut icon, unread);
    }

    Ok(BadgeImage {
        rgba: icon.into_raw(),
        width: ICON_SIZE,
        height: ICON_SIZE,
    })
}

fn draw_badge(icon: &mut RgbaImage, unread: u32) {
    const CENTER_X: i32 = 48;
    const CENTER_Y: i32 = 16;
    const RADIUS: i32 = 16;

    for y in 0..ICON_SIZE as i32 {
        for x in 0..ICON_SIZE as i32 {
            let dx = x - CENTER_X;
            let dy = y - CENTER_Y;
            if dx * dx + dy * dy <= RADIUS * RADIUS {
                icon.put_pixel(x as u32, y as u32, Rgba([220, 38, 38, 255]));
            }
        }
    }

    let label = if unread > 99 {
        "99+".to_owned()
    } else {
        unread.to_string()
    };
    draw_text(icon, &label, CENTER_X, CENTER_Y);
}

fn draw_text(icon: &mut RgbaImage, text: &str, center_x: i32, center_y: i32) {
    const SCALE: i32 = 2;
    const GLYPH_WIDTH: i32 = 3;
    const GLYPH_HEIGHT: i32 = 5;
    const GAP: i32 = 1;

    let chars: Vec<char> = text.chars().collect();
    let text_width = (chars.len() as i32 * GLYPH_WIDTH + (chars.len() as i32 - 1) * GAP) * SCALE;
    let start_x = center_x - text_width / 2;
    let start_y = center_y - (GLYPH_HEIGHT * SCALE) / 2;

    for (index, character) in chars.iter().enumerate() {
        let glyph = glyph(*character);
        let glyph_x = start_x + index as i32 * (GLYPH_WIDTH + GAP) * SCALE;
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..GLYPH_WIDTH {
                if bits & (1 << (GLYPH_WIDTH - column - 1)) == 0 {
                    continue;
                }
                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        let x = glyph_x + column * SCALE + sx;
                        let y = start_y + row as i32 * SCALE + sy;
                        if x >= 0 && y >= 0 && x < ICON_SIZE as i32 && y < ICON_SIZE as i32 {
                            icon.put_pixel(x as u32, y as u32, Rgba([255, 255, 255, 255]));
                        }
                    }
                }
            }
        }
    }
}

fn glyph(character: char) -> [u8; 5] {
    match character {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        _ => [0; 5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_icon_and_badges_have_stable_dimensions() {
        for count in [0, 1, 99, 100, u32::MAX] {
            let icon = render(count).unwrap();
            assert_eq!((icon.width, icon.height), (64, 64));
            assert_eq!(icon.rgba.len(), 64 * 64 * 4);
        }
    }

    #[test]
    fn zero_resets_to_base_icon() {
        assert_eq!(render(0).unwrap(), render(0).unwrap());
        assert_ne!(render(0).unwrap(), render(1).unwrap());
    }

    #[test]
    fn excessive_counts_are_bounded_to_same_badge() {
        assert_eq!(render(100).unwrap(), render(u32::MAX).unwrap());
    }

    #[test]
    fn badge_center_is_deterministic_red() {
        let icon = render(3).unwrap();
        let offset = ((16 * icon.width + 34) * 4) as usize;
        assert_eq!(&icon.rgba[offset..offset + 4], &[220, 38, 38, 255]);
    }

    #[test]
    fn embedded_icon_is_valid_png() {
        assert!(image::load_from_memory(BASE_ICON).is_ok());
    }
}
