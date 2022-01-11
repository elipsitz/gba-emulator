use super::{BackgroundBuffer, ObjectBuffer, ObjectBufferEntry};
use crate::{
    mem::Memory,
    ppu::{color::Color15, PIXELS_WIDTH},
    Gba,
};

impl Gba {
    /// Do final composition of a scanline and write it to the screenbuffer.
    pub(super) fn ppu_compose_scanline(
        &mut self,
        object_buffer: &ObjectBuffer,
        background_buffers: &[BackgroundBuffer; 4],
        background_indices: &mut [usize],
    ) {
        let framebuffer_offset = PIXELS_WIDTH * (self.ppu.vcount as usize);
        let backdrop_color = Color15(self.ppu.palette.read_16(0));

        // Sort backgrounds.
        background_indices.sort_by_key(|&x| self.ppu.bgcnt[x].priority);

        for x in 0..PIXELS_WIDTH {
            let obj = &object_buffer[x];
            let color = self.compose_pixel(
                background_buffers,
                background_indices,
                obj,
                x,
                backdrop_color,
            );
            self.ppu.framebuffer[framebuffer_offset + x] = color.as_argb();
        }
    }

    fn compose_pixel(
        &mut self,
        bg_buffers: &[BackgroundBuffer; 4],
        bg_indices: &[usize],
        obj: &ObjectBufferEntry,
        x: usize,
        backdrop_color: Color15,
    ) -> Color15 {
        // TODO: implement more complex object/background priority interactions.
        // To support blending, we need to find the top two non-transparent layers.
        let (top, _bottom) = {
            // First loop at backgrounds and backdrops.
            let backdrop = Layer::backdrop(backdrop_color);
            let mut bg_iter = bg_indices
                .iter()
                .filter(|&&i| !bg_buffers[i][x].transparent());
            let mut top = bg_iter.next().map_or(backdrop, |&i| {
                Layer::background(i, bg_buffers[i][x], self.ppu.bgcnt[i].priority)
            });
            let mut bottom = bg_iter.next().map_or(backdrop, |&i| {
                Layer::background(i, bg_buffers[i][x], self.ppu.bgcnt[i].priority)
            });

            // Now see if there's an object that goes on top.
            if self.ppu.dispcnt.display_obj && !obj.color.transparent() {
                if obj.priority <= top.priority {
                    bottom = top;
                    top = Layer::object(obj.color, obj.priority);
                } else if obj.priority <= bottom.priority {
                    bottom = Layer::object(obj.color, obj.priority);
                }
            }

            (top, bottom)
        };

        top.color
    }
}

#[derive(Copy, Clone)]
struct Layer {
    /// Layer kind. Matches with the bitfield in BLDCNT.
    kind: usize,
    color: Color15,
    priority: u16,
}

impl Layer {
    fn backdrop(color: Color15) -> Layer {
        Layer {
            kind: KIND_BACKDROP,
            color,
            priority: u16::MAX,
        }
    }

    fn background(index: usize, color: Color15, priority: u16) -> Layer {
        Layer {
            kind: index,
            color,
            priority,
        }
    }

    fn object(color: Color15, priority: u16) -> Layer {
        Layer {
            kind: KIND_OBJ,
            color,
            priority,
        }
    }
}

const KIND_OBJ: usize = 4;
const KIND_BACKDROP: usize = 5;
