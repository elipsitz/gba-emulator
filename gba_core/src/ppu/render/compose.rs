use super::{BackgroundBuffer, ObjectBuffer, ObjectBufferEntry};
use crate::{
    mem::Memory,
    ppu::{
        color::Color15,
        registers::{BlendMode, WindowControl},
        PIXELS_WIDTH,
    },
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
        // First determine the active window.
        let window = if !self.ppu.dispcnt.windows_enabled() {
            WindowControl::none()
        } else {
            if self.ppu.dispcnt.window_display[0]
                && self.ppu.window_scanline_active[0]
                && self.ppu.win_h[0].test(x)
            {
                self.ppu.win_in.win0
            } else if self.ppu.dispcnt.window_display[1]
                && self.ppu.window_scanline_active[1]
                && self.ppu.win_h[1].test(x)
            {
                self.ppu.win_in.win1
            } else if self.ppu.dispcnt.obj_window_display && obj.window {
                self.ppu.win_out.win_obj
            } else {
                self.ppu.win_out.win_out
            }
        };

        // TODO: implement more complex object/background priority interactions.
        // To support blending, we need to find the top two non-transparent layers.
        let (top, bottom) = {
            // First loop at backgrounds and backdrops.
            let backdrop = Layer::backdrop(backdrop_color);
            let mut bg_iter = bg_indices
                .iter()
                .filter(|&&i| !bg_buffers[i][x].transparent() && window.layer[i]);
            let mut top = bg_iter.next().map_or(backdrop, |&i| {
                Layer::background(i, bg_buffers[i][x], self.ppu.bgcnt[i].priority)
            });
            let mut bottom = bg_iter.next().map_or(backdrop, |&i| {
                Layer::background(i, bg_buffers[i][x], self.ppu.bgcnt[i].priority)
            });

            // Now see if there's an object that goes on top.
            if self.ppu.dispcnt.display_obj && !obj.color.transparent() && window.layer[KIND_OBJ] {
                if obj.priority <= top.priority {
                    bottom = top;
                    top = Layer::object(obj.color, obj.priority);
                } else if obj.priority <= bottom.priority {
                    bottom = Layer::object(obj.color, obj.priority);
                }
            }

            (top, bottom)
        };

        // Whether the top layer is a blended object (has special behavior).
        let object_blend = (top.kind == KIND_OBJ) && obj.blend;
        if !(window.blend || object_blend) {
            // No blending in this window.
            return top.color;
        }

        let blend_mode = if object_blend {
            BlendMode::Normal
        } else {
            self.ppu.bldcnt.mode
        };
        let blend_top = self.ppu.bldcnt.top[top.kind];
        let blend_bottom = self.ppu.bldcnt.bottom[bottom.kind];
        let blend_enabled = (blend_top && blend_mode != BlendMode::None) || object_blend;

        if blend_enabled {
            match blend_mode {
                BlendMode::Normal if blend_bottom => Color15::blend(
                    top.color,
                    bottom.color,
                    self.ppu.bldalpha.top,
                    self.ppu.bldalpha.bottom,
                ),
                BlendMode::White => {
                    let fade = self.ppu.bldy.fade.min(16);
                    Color15::blend(top.color, Color15::WHITE, 16 - fade, fade)
                }
                BlendMode::Black => {
                    let fade = self.ppu.bldy.fade.min(16);
                    Color15::blend(top.color, Color15::BLACK, 16 - fade, fade)
                }
                _ => top.color,
            }
        } else {
            // No blending. Use the top layer.
            top.color
        }
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
