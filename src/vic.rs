use emu::*;
use memc64::*;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Renderer;

// Color palette - from http://www.pepto.de/projects/colorvic/2001/s
static COLORS: &'static [Color] = &[
    Color::RGB(0x00, 0x00, 0x00), // 0: black
    Color::RGB(0xff, 0xff, 0xff), // 1: white
    Color::RGB(0x68, 0x37, 0x2b), // 2: red
    Color::RGB(0x70, 0xa4, 0xb2), // 3: cyan
    Color::RGB(0x6f, 0x3d, 0x86), // 4: purple
    Color::RGB(0x58, 0x8d, 0x43), // 5: green
    Color::RGB(0x35, 0x28, 0x79), // 6: blue
    Color::RGB(0xb8, 0xc7, 0x6f), // 7: yellow
    Color::RGB(0x6f, 0x4f, 0x25), // 8: orange
    Color::RGB(0x43, 0x39, 0x00), // 9: brown
    Color::RGB(0x9a, 0x67, 0x59), // 10: light red
    Color::RGB(0x44, 0x44, 0x44), // 11: dark gray
    Color::RGB(0x6c, 0x6c, 0x6c), // 12: gray
    Color::RGB(0x9a, 0xd2, 0x84), // 13: light green
    Color::RGB(0x6c, 0x5e, 0xb5), // 14: light blue
    Color::RGB(0x95, 0x95, 0x95), // 15: light gray
];

fn color_from_reg(mem: &VicIO, clr: u16) -> Color {
    COLORS[(mem.vic_read_register(clr) % 16) as usize]
}

pub const VIC_SCROLX: u16 = 0xD016;
pub const VIC_SCROLY: u16 = 0xD011;
pub const VIC_RASTER: u16 = 0xD012;
pub const VIC_SPRITE_ENABLED: u16 = 0xD015;
pub const VIC_MEMORY: u16 = 0xD018;

pub const VIC_BORDER_COLOR: u16 = 0xD020;
pub const VIC_BACKGROUND_COLOR_0: u16 = 0xD021;
pub const VIC_BACKGROUND_COLOR_1: u16 = 0xD022;
pub const VIC_BACKGROUND_COLOR_2: u16 = 0xD023;
//pub const VIC_BACKGROUND_COLOR_3: u16 = 0xD024;

pub const VIC_SPRITE_POS_BASE: u16 = 0xD000;
pub const VIC_SPRITE_CLR_BASE: u16 = 0xD027;
pub const VIC_SPRITE_XPOS_MSB: u16 = 0xD010;
pub const VIC_SPRITE_PTR_OFFSET: u16 = 0x03F8;

pub const VIC_SPRITE_POS_X_OFFSET: i32 = 24;
pub const VIC_SPRITE_POS_Y_OFFSET: i32 = 50;

pub const VIC_COLOR_RAM: u16 = 0xD800;

pub struct VIC {
    renderer: Renderer<'static>,
    cycles_per_line: u16,
    max_lines: u16,
    scale: u8,
    curr_line: u16,
    curr_cycle: u16,
}

impl VIC {
    pub fn new(r: Renderer<'static>, scale: u8) -> VIC {
        let (cycles_per_line, max_lines) = (65, 263); // NTSC
        VIC {
            renderer: r,
            cycles_per_line: cycles_per_line,
            max_lines: max_lines,
            scale: scale,
            curr_line: 0,
            curr_cycle: 0,
        }
    }

    pub fn init(&mut self, mem: &mut VicIO) {
        // Set up Char ROM and Video Matrix start locations
        mem.vic_write_register(VIC_MEMORY, 0x14);
        mem.vic_write_register(VIC_SCROLX, 0x08);
    }

    pub fn clock(&mut self, mem: &mut VicIO) {
        if self.curr_cycle == 0 {
            // Update RASTER - Kernal init waits for this register to go to 0
            mem.vic_write_register(VIC_RASTER, (self.curr_line & 0xFF) as u8);
            let mut b = mem.vic_read_register(VIC_SCROLY);
            b = b & 0x7F;
            if self.curr_line > 255 {
                b = b | 0x80; // Or in MSB if needed
            }
            mem.vic_write_register(VIC_SCROLY, b);
        }

        self.curr_cycle = (self.curr_cycle + 1) % self.cycles_per_line;
        if self.curr_cycle == 0 {
            self.curr_line = (self.curr_line + 1) % self.max_lines;
        }
    }

    pub fn refresh(&mut self, mem: &VicIO) {
        // Clear screen
        self.renderer
            .set_draw_color(color_from_reg(mem, VIC_BORDER_COLOR));
        self.renderer.clear();

        // Clear main window background
        let rc_main = Rect::new(
            BORDER_WIDTH as i32 * self.scale as i32,
            BORDER_HEIGHT as i32 * self.scale as i32,
            VIEWABLE_WIDTH * self.scale as u32,
            VIEWABLE_HEIGHT * self.scale as u32,
        );
        self.renderer
            .set_draw_color(color_from_reg(mem, VIC_BACKGROUND_COLOR_0));
        self.renderer.fill_rect(rc_main).unwrap();

        let bitmap_mode = (mem.vic_read_register(VIC_SCROLY) & 0x20) > 0; // Bit 5 is 1

        match bitmap_mode {
            true => self.draw_bitmap(mem),
            false => self.draw_text(mem),
        }

        self.draw_sprites(mem);

        self.renderer.present();
    }

    fn draw_text(&mut self, mem: &VicIO) {
        // Draw text contents of video matrix
        let vmatrix_start: usize = ((mem.vic_read_register(VIC_MEMORY) >> 4) as usize) * 1024;
        let charrom_start: usize =
            (((mem.vic_read_register(VIC_MEMORY) & 0x0E) >> 1) as usize) * 2048;
        let multi_color: bool = (mem.vic_read_register(VIC_SCROLX) & 0x10) > 0;
        let extended_color: bool = (mem.vic_read_register(VIC_SCROLY) & 0x40) > 0;

        let c0 = mem.vic_read_register(VIC_BACKGROUND_COLOR_0) as usize;
        let c1 = mem.vic_read_register(VIC_BACKGROUND_COLOR_1) as usize;
        let c2 = mem.vic_read_register(VIC_BACKGROUND_COLOR_2) as usize;

        for row in 0..25 {
            for col in 0..40 {
                let char_index = vmatrix_start + col + (row * 40);
                let (mut char_rom_index, fg_color) = mem.vic_read_vm(char_index as u16);
                let char_start = charrom_start + ((char_rom_index as usize) * 8);

                let bg_color = if extended_color {
                    let bg_clr_index = char_rom_index / 64;
                    mem.vic_read_register(VIC_BACKGROUND_COLOR_0 + bg_clr_index as u16) as usize
                } else {
                    mem.vic_read_register(VIC_BACKGROUND_COLOR_0) as usize
                };

                if multi_color && (fg_color >= 8) {
                    let c3 = (fg_color & 0x07) as usize;
                    self.draw_char_multicolor(row, col, mem, char_start, c0, c1, c2, c3);
                } else {
                    self.draw_char(row, col, mem, char_start, bg_color, fg_color as usize);
                }
            }
        }
    }

    fn draw_sprites(&mut self, mem: &VicIO) {
        // Draw sprites
        let vmatrix_start: usize = ((mem.vic_read_register(VIC_MEMORY) >> 4) as usize) * 1024;
        let sprites_enabled = mem.vic_read_register(VIC_SPRITE_ENABLED);
        let sprite_ptrs = vmatrix_start as u16 + VIC_SPRITE_PTR_OFFSET;

        for sprite in 0..8 {
            if (sprites_enabled & (1 << sprite)) > 0 {
                let sprite_addr = mem.vic_read_byte(sprite_ptrs + sprite) as u16 * 64;
                let sprite_clr = mem.vic_read_register(VIC_SPRITE_CLR_BASE + sprite) % 16;
                let mut sprite_x = mem.vic_read_register(VIC_SPRITE_POS_BASE + (sprite * 2)) as u16;
                let sprite_x_msb = mem.vic_read_register(VIC_SPRITE_XPOS_MSB);
                if (sprite_x_msb & (1 << sprite)) > 0 {
                    sprite_x |= 0x100
                };
                let sprite_y = mem.vic_read_register(VIC_SPRITE_POS_BASE + (sprite * 2) + 1);
                self.renderer.set_draw_color(COLORS[sprite_clr as usize]);
                self.draw_sprite(sprite_addr, sprite_x, sprite_y, mem)
            }
        }
    }

    fn draw_bitmap(&mut self, mem: &VicIO) {
        let bitmap_start: usize = ((mem.vic_read_register(VIC_MEMORY) & 0x08) as usize) * 1024;
        let color_start: usize = ((mem.vic_read_register(VIC_MEMORY) >> 4) as usize) * 1024;
        let multi_color: bool = (mem.vic_read_register(VIC_SCROLX) & 0x10) > 0;

        for row in 0..25 {
            for col in 0..40 {
                let char_start = bitmap_start + (col * 8) + (row * 40 * 8);
                let (color_byte, color_mem_byte) =
                    mem.vic_read_vm((color_start + col + (row * 40)) as u16);
                let bg_color = color_byte & 0x0F;
                let fg_color = (color_byte & 0xF0) >> 4;

                if multi_color {
                    let c0 = mem.vic_read_register(VIC_BACKGROUND_COLOR_0) as usize;
                    let c1 = (color_byte >> 4) as usize;
                    let c2 = (color_byte & 0x0F) as usize;
                    let c3 = (color_mem_byte & 0x0F) as usize;
                    self.draw_char_multicolor(row, col, mem, char_start, c0, c1, c2, c3);
                } else {
                    self.draw_char(
                        row,
                        col,
                        mem,
                        char_start,
                        bg_color as usize,
                        fg_color as usize,
                    );
                }
            }
        }
    }

    fn draw_sprite(&mut self, sprite_addr: u16, sprite_x: u16, sprite_y: u8, mem: &VicIO) {
        //println!("draw sprite at addr: {:04X} => ", sprite_addr);
        let mut rc = Rect::new(0, 0, self.scale as u32, self.scale as u32);
        for y in 0..21 {
            for c in 0..3 {
                let currb = mem.vic_read_byte(sprite_addr + (y * 3) + c);
                //println!("  y={}, c={}: {:02X} ", y, c, currb);
                for bit in 0..8 {
                    if (currb & (1 << (7 - bit))) > 0 {
                        let scr_x =
                            BORDER_WIDTH as i32 + (sprite_x as i32) + (c as i32 * 8) + (bit as i32)
                                - VIC_SPRITE_POS_X_OFFSET;
                        let scr_y = BORDER_HEIGHT as i32 + (sprite_y as i32) + (y as i32)
                            - VIC_SPRITE_POS_Y_OFFSET;
                        rc.set_x(scr_x * self.scale as i32);
                        rc.set_y(scr_y * self.scale as i32);
                        //println!("    ({},{})", rc.x(), rc.y());
                        self.renderer.fill_rect(rc).unwrap();
                    }
                }
            }
        }
        //println!("");
    }

    fn draw_char(
        &mut self,
        row: usize,
        col: usize,
        mem: &VicIO,
        char_start: usize,
        bg_color: usize,
        fg_color: usize,
    ) {
        let mut rc = Rect::new(0, 0, self.scale as u32, self.scale as u32);
        let mut rc_bg = Rect::new(0, 0, 8 * self.scale as u32, 8 * self.scale as u32);

        let scr_x = BORDER_WIDTH as i32 + ((col as i32) * 8);
        let scr_y = BORDER_HEIGHT as i32 + ((row as i32) * 8);
        rc_bg.set_x(scr_x * self.scale as i32);
        rc_bg.set_y(scr_y * self.scale as i32);
        self.renderer.set_draw_color(COLORS[bg_color % 16]);
        self.renderer.fill_rect(rc_bg).unwrap();

        self.renderer.set_draw_color(COLORS[fg_color]);

        for y in 0..8 {
            let currb = mem.vic_read_byte(char_start as u16 + y);
            for bit in 0..8 {
                if (currb & (1 << (7 - bit))) > 0 {
                    let scr_x = BORDER_WIDTH as i32 + ((col as i32) * 8) + (bit as i32);
                    let scr_y = BORDER_HEIGHT as i32 + ((row as i32) * 8) + (y as i32);

                    rc.set_x(scr_x * self.scale as i32);
                    rc.set_y(scr_y * self.scale as i32);
                    self.renderer.fill_rect(rc).unwrap();
                }
            }
        }
    }

    fn draw_char_multicolor(
        &mut self,
        row: usize,
        col: usize,
        mem: &VicIO,
        char_start: usize,
        c0: usize,
        c1: usize,
        c2: usize,
        c3: usize,
    ) {
        let mut rc = Rect::new(0, 0, 2 * self.scale as u32, self.scale as u32);

        for y in 0..8 {
            let currb = mem.vic_read_byte(char_start as u16 + y);
            for bit in 0..4 {
                let m = (currb >> (6 - (bit * 2))) & 0x03;
                let c = match m {
                    0 => c0,
                    1 => c1,
                    2 => c2,
                    _ => c3,
                };

                let scr_x = BORDER_WIDTH as i32 + ((col as i32) * 8) + ((2 * bit) as i32);
                let scr_y = BORDER_HEIGHT as i32 + ((row as i32) * 8) + (y as i32);

                rc.set_x(scr_x * self.scale as i32);
                rc.set_y(scr_y * self.scale as i32);

                self.renderer.set_draw_color(COLORS[c % 16]);
                self.renderer.fill_rect(rc).unwrap();
            }
        }
    }
}
