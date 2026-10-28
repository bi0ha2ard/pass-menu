use std::rc::Rc;

use ab_glyph::{FontRef, Glyph, PxScaleFont, ScaleFont, point};
use softbuffer::Buffer;
use winit::{event_loop::OwnedDisplayHandle, window::Window};

type Buf<'a> = Buffer<'a, OwnedDisplayHandle, Rc<Window>>;

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | (b as u32)
}

const fn mix(dest: u32, src: u32, amount: f32) -> u32 {
    if amount >= 1.0 {
        return dest;
    }
    if amount < 0.01 {
        return src;
    }

    let [b1, g1, r1, _] = dest.to_le_bytes();
    let [b2, g2, r2, _] = src.to_le_bytes();

    let r = (r1 as f32 * amount + r2 as f32 * (1.0 - amount)).clamp(0.0, 255.0) as u8;
    let g = (g1 as f32 * amount + g2 as f32 * (1.0 - amount)).clamp(0.0, 255.0) as u8;
    let b = (b1 as f32 * amount + b2 as f32 * (1.0 - amount)).clamp(0.0, 255.0) as u8;
    rgb(r, g, b)
}

pub const BG: u32 = rgb(0x18, 0x18, 0x18);
pub const TEXT: u32 = rgb(0xaa, 0xaa, 0xaa);

pub fn rect(buf: &mut Buf, x0: u32, y0: u32, x1: u32, y1: u32, color: u32) {
    let width = buf.width().get();
    let height = buf.height().get();
    let x0 = x0.clamp(0, width);
    let x1 = x1.clamp(0, width);
    let y0 = y0.clamp(0, height);
    let y1 = y1.clamp(0, height);

    assert!(x0 <= x1);
    assert!(y0 <= y1);

    for y in y0..y1 {
        for x in x0..x1 {
            buf[y as usize * width as usize + x as usize] = color;
        }
    }
}

const fn in_bounds(width: u32, height: u32, x: f32, y: f32) -> Option<(u32, u32)> {
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let x = x as u32;
    let y = y as u32;
    if x < width && y < height {
        return Some((x, y));
    }
    None
}

const TEXT_BOX_PADDING: u32 = 5;
const CURSOR_THICKNESS: u32 = 4;
pub const FONT_SIZE: u32 = 40;
pub const TEXT_BOX_HEIGHT: u32 = FONT_SIZE + TEXT_BOX_PADDING * 2;

pub fn draw_text_box(
    buf: &mut Buf,
    font: &PxScaleFont<&FontRef<'_>>,
    text: &str,
    x: u32,
    y: u32,
    width: u32,
    color: u32,
    bg: u32,
) {
    rect(buf, x, y, x + width, y + TEXT_BOX_HEIGHT, bg);
    let _caret = draw_text(
        buf,
        font,
        text,
        x + TEXT_BOX_PADDING,
        y + TEXT_BOX_PADDING as u32,
        color,
        x + width.saturating_sub(2 * TEXT_BOX_PADDING),
    );
    /*
    if let Some(caret) = caret {
        rect(
            buf,
            caret,
            y + TEXT_BOX_PADDING,
            caret + CURSOR_THICKNESS,
            y + TEXT_BOX_PADDING + font.height() as u32,
            color,
        );
    }
    */
}

pub fn draw_label(
    buf: &mut Buf,
    font: &PxScaleFont<&FontRef<'_>>,
    text: &str,
    x: u32,
    y: u32,
    clip_width: u32,
    color: u32,
    bg: u32,
) -> u32 {
    let text_width = measure_text(font, text);
    let required_width = (text_width + 2 * TEXT_BOX_PADDING).min(clip_width);
    let text_clip = required_width.saturating_sub(TEXT_BOX_PADDING);
    rect(buf, x, y, x + required_width, y + TEXT_BOX_HEIGHT, bg);
    draw_text(
        buf,
        font,
        text,
        x + TEXT_BOX_PADDING,
        y + TEXT_BOX_PADDING,
        color,
        x + text_clip,
    );
    required_width
}

pub fn measure_text(font: &PxScaleFont<&FontRef<'_>>, text: &str) -> u32 {
    let mut caret = point(0.0, 0.0);

    let mut last_glyph: Option<Glyph> = None;

    for c in text.chars() {
        let glyph = font.scaled_glyph(c);
        if let Some(previous) = last_glyph.take() {
            caret.x += font.kern(previous.id, glyph.id);
        }
        caret.x += font.h_advance(glyph.id);
    }

    caret.x.ceil() as u32
}

pub fn draw_text(
    buf: &mut Buf,
    font: &PxScaleFont<&FontRef<'_>>,
    text: &str,
    x: u32,
    y: u32,
    color: u32,
    clip: u32,
) -> Option<u32> {
    let width = buf.width().get();
    let clip_width = buf.width().get().min(clip);
    let height = buf.height().get();

    let mut caret = point(x as f32, y as f32 + font.ascent());

    let mut last_glyph: Option<Glyph> = None;

    for c in text.chars() {
        let mut glyph = font.scaled_glyph(c);
        if let Some(previous) = last_glyph.take() {
            caret.x += font.kern(previous.id, glyph.id);
        }
        glyph.position = caret;
        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if let Some(g) = font.outline_glyph(glyph) {
            let bounds = g.px_bounds();
            if bounds.max.x as u32 > clip_width {
                return None;
            }
            g.draw(|x, y, c| {
                if c <= 0.01 {
                    return;
                }
                if let Some((x, y)) = in_bounds(
                    clip_width,
                    height,
                    x as f32 + bounds.min.x,
                    y as f32 + bounds.min.y,
                ) {
                    let idx = (y * width + x) as usize;
                    assert!(idx < buf.len());
                    buf[idx] = mix(color, buf[idx], c);
                }
            });
        }

        if caret.x as u32 > clip_width {
            return None;
        }
    }

    Some(caret.x as u32)
}
