use std::rc::Rc;

use softbuffer::Buffer;
use winit::{event_loop::OwnedDisplayHandle, window::Window};

type Buf<'a> = Buffer<'a, OwnedDisplayHandle, Rc<Window>>;

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
