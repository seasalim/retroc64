use keyboard::*;
use memory::*;
use sdl2::keyboard::Scancode;
use utils::*;
use vic::*;

//pub const CIA1_ICR: u16 = 0xDC0D;
pub const CIA2_DATA_PORT_A: u16 = 0xDD00;
pub const DATA_DIRECTION_PORT: u16 = 0;
pub const IO_PORT: u16 = 1;

const LORAM: u8 = 0;
const HIRAM: u8 = 1;
const CHAREN: u8 = 2;

pub struct MemC64 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    io: Vec<u8>,
    keys: Vec<Scancode>,
}

impl MemC64 {
    pub fn new() -> MemC64 {
        let rom = vec![0; 64 * 1024];
        let io = vec![0; 4 * 1024];
        let mut ram = vec![0; 64 * 1024];
        let keys = Vec::new();

        ram[DATA_DIRECTION_PORT as usize] = 0b101111;
        ram[IO_PORT as usize] = 0b111;

        MemC64 {
            rom: rom,
            ram: ram,
            io: io,
            keys: keys,
        }
    }

    pub fn load_rom(&mut self, buf: &Vec<u8>, addr: u16) {
        debug!("Loading ROM at addr ${:04X}", addr);
        for ix in 0..buf.len() {
            self.rom[addr as usize + ix] = buf[ix];
        }
    }

    pub fn load_ram(&mut self, buf: &Vec<u8>, addr: u16) {
        debug!("Loading RAM at addr ${:04X}", addr);
        for ix in 0..buf.len() {
            self.ram[addr as usize + ix] = buf[ix];
        }
    }

    pub fn refresh(&mut self, keys: Vec<Scancode>) {
        self.keys = keys;
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xDC01 => kbd_scancode(&self.keys, !self.io[0xDC00 - 0xD000]),
            0xD000...0xDFFF => self.io[addr as usize - 0xD000],
            _ => panic!("Register read out-of-bounds: ${:04X}", addr),
        }
    }

    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            0xD000...0xDFFF => self.io[addr as usize - 0xD000] = val,
            _ => panic!("Register write out-of-bounds: ${:04X}", addr),
        }
    }
}

pub trait CiaIO {
    fn cia_read_register(&self, addr: u16) -> u8;
    fn cia_write_register(&mut self, addr: u16, val: u8);
}

impl CiaIO for MemC64 {
    fn cia_read_register(&self, addr: u16) -> u8 {
        match addr {
            0xDC00...0xDDFF => self.read_register(addr),
            _ => panic!("CIA out-of-range register read: ${:04X}", addr),
        }
    }

    fn cia_write_register(&mut self, addr: u16, val: u8) {
        match addr {
            0xDC00...0xDDFF => self.write_register(addr, val),
            _ => panic!("CIA  out-of-range register write: ${:04X}", addr),
        }
    }
}

pub trait VicIO {
    fn vic_read_byte(&self, addr: u16) -> u8;
    fn vic_read_register(&self, addr: u16) -> u8;
    fn vic_write_register(&mut self, addr: u16, val: u8);
    fn vic_read_vm(&self, addr: u16) -> (u8, u8);
}

impl VicIO for MemC64 {
    fn vic_read_byte(&self, addr: u16) -> u8 {
        assert!(addr < (16 * 1024)); // VIC only sees 16K
        let bank = 3 - (self.read_register(CIA2_DATA_PORT_A) & 0x03) as u16;
        let val = match addr {
            0x1000...0x1FFF if (bank == 0) || (bank == 2) => self.rom[(addr + 0xC000) as usize], // Char ROM
            _ => self.ram[(addr + (bank * 0x4000)) as usize],
        };
        // println!(
        //     "VIC read byte: addr: ${:04X} bank: {:02X} full addr: ${:04X} = ${:02X}",
        //     addr,
        //     bank,
        //     addr + (bank * 0x4000),
        //     val
        // );
        val
    }

    fn vic_read_vm(&self, addr: u16) -> (u8, u8) {
        let bank = 3 - (self.read_register(CIA2_DATA_PORT_A) & 0x03) as u16;
        let ch = self.ram[(addr + (bank * 0x4000)) as usize];
        let clr_ix = addr % 0x0400;
        let clr = self.io[(VIC_COLOR_RAM - 0xD000 + clr_ix) as usize] & 0x0F;

        (ch, clr)
    }

    fn vic_read_register(&self, addr: u16) -> u8 {
        match addr {
            0xD000...0xD02E => self.read_register(addr),
            _ => panic!("VIC out-of-range register read: ${:04X}", addr),
        }
    }

    fn vic_write_register(&mut self, addr: u16, val: u8) {
        match addr {
            0xD000...0xD02E => self.write_register(addr, val),
            _ => panic!("VIC out-of-range register write: ${:04X}", addr),
        }
    }
}

impl MemIO for MemC64 {
    fn memdump(&self, from: usize, bytes: usize) {
        let mut buf = vec![0; bytes];
        for i in 0..bytes {
            buf[i] = self.read_byte((from + i) as u16);
        }
        hexdump(&buf, 0, bytes)
    }

    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            IO_PORT => {
                debug!(
                    "Read IO_PORT (${}): ${:02X}",
                    IO_PORT, self.ram[IO_PORT as usize]
                );
                self.ram[IO_PORT as usize]
            }
            DATA_DIRECTION_PORT => {
                debug!(
                    "Read DATA_DIRECTION_PORT (${}): ${:02X}",
                    DATA_DIRECTION_PORT, self.ram[DATA_DIRECTION_PORT as usize]
                );
                self.ram[DATA_DIRECTION_PORT as usize]
            }
            0xA000...0xBFFF => {
                if (self.ram[IO_PORT as usize] & (1 << LORAM)) > 0 {
                    self.rom[addr as usize]
                } else {
                    self.ram[addr as usize]
                }
            }
            0xD000...0xDFFF => {
                if (self.ram[IO_PORT as usize] & (1 << CHAREN)) > 0 {
                    self.read_register(addr)
                } else {
                    self.rom[addr as usize]
                }
            }
            0xE000...0xFFFF => {
                if (self.ram[IO_PORT as usize] & (1 << HIRAM)) > 0 {
                    self.rom[addr as usize]
                } else {
                    self.ram[addr as usize]
                }
            }
            _ => self.ram[addr as usize],
        }
    }

    fn read_word(&self, addr: u16) -> u16 {
        let lobyte = self.read_byte(addr) as u16;
        let hibyte = self.read_byte(addr + 1) as u16;
        (hibyte << 8) | lobyte
    }

    fn write_byte(&mut self, addr: u16, val: u8) {
        match addr {
            IO_PORT => {
                debug!("Write IO_PORT (${}): ${:02X}", IO_PORT, val);
                self.ram[IO_PORT as usize] = val;
            }
            DATA_DIRECTION_PORT => {
                debug!(
                    "Write DATA_DIRECTION_PORT (${}): ${:02X}",
                    DATA_DIRECTION_PORT, val
                );
                self.ram[DATA_DIRECTION_PORT as usize] = val;
            }
            0xD000...0xDFFF => {
                if (self.ram[IO_PORT as usize] & (1 << CHAREN)) > 0 {
                    self.write_register(addr, val);
                } else {
                    self.ram[addr as usize] = val;
                }
            }
            _ => self.ram[addr as usize] = val,
        }
    }
}
