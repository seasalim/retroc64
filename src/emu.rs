use sdl2;
use sdl2::keyboard::Scancode;
use sdl2::render::Renderer;
use std::fs;
use std::io;
use std::io::Write;
use time::{Duration, PreciseTime};

use cpu::*;
use dasm::*;
use memc64::*;
use memory::*;
use utils::*;
use vic::*;

pub const BORDER_WIDTH: u32 = 32;
pub const BORDER_HEIGHT: u32 = 16;
pub const VIEWABLE_WIDTH: u32 = 320;
pub const VIEWABLE_HEIGHT: u32 = 200;
pub const WINDOW_WIDTH: u32 = VIEWABLE_WIDTH + (BORDER_WIDTH * 2);
pub const WINDOW_HEIGHT: u32 = VIEWABLE_HEIGHT + (BORDER_HEIGHT * 2);

pub struct C64 {
    cpu: CPU,
    vic: VIC,
    mem: MemC64,
    sdl: sdl2::Sdl,
}

impl C64 {
    pub fn new() -> C64 {
        let cpu = CPU::new();
        let mem = MemC64::new();

        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();
        let scale = 2;

        let window = video
            .window("C64", WINDOW_WIDTH * scale, WINDOW_HEIGHT * scale)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let renderer: Renderer<'static> = window.renderer().build().unwrap();
        let vic = VIC::new(renderer, scale as u8);

        C64 {
            cpu: cpu,
            vic: vic,
            mem: mem,
            sdl: sdl,
        }
    }

    pub fn start(&mut self, start_addr: u16, debug: bool) {
        for &(rname, raddr) in &[
            ("roms/basic", 0xA000),
            ("roms/chargen", 0xD000),
            ("roms/kernal", 0xE000),
        ] {
            self.mem.load_rom(&load_file(rname).unwrap(), raddr);
        }

        self.cpu.set_pc(start_addr);
        self.vic.init(&mut self.mem);

        self.execute(debug);
    }

    fn execute(&mut self, debug: bool) {
        let mut event_pump = self.sdl.event_pump().unwrap();
        let mut break_set = debug;

        'running: loop {
            let tick = PreciseTime::now();

            // Run for up to 15 ms or a breakpoint hit
            while !break_set && (tick.to(PreciseTime::now()) < Duration::milliseconds(15)) {
                let mut step_cycles: u8 = 0;
                break_set = self.do_single_step(&mut step_cycles);
                self.vic.clock(&mut self.mem);
            }

            // Pump events and trigger interrupt if we have not broken yet
            if !break_set {
                event_pump.pump_events();
                if event_pump
                    .keyboard_state()
                    .is_scancode_pressed(Scancode::Escape)
                {
                    println!("Breaking... ${:04X}", self.cpu.get_pc());
                    break_set = true;
                } else {
                    let keys: Vec<Scancode> =
                        event_pump.keyboard_state().pressed_scancodes().collect();
                    self.mem.refresh(keys);
                    self.vic.refresh(&self.mem);
                    self.cpu.trigger_irq(&mut self.mem);
                }
            }

            // Handle break state
            if break_set {
                println!("");
                let mut ip = self.cpu.get_pc();

                for index in 0..3 {
                    match disassemble_step(&self.mem, &mut ip) {
                        Ok(s) => {
                            if index == 0 {
                                println!("* {}", s)
                            } else {
                                println!("  {}", s)
                            }
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                            break;
                        }
                    }
                }

                print!("\nCommand (? for help) : ");
                io::stdout().flush().ok().expect("Could not flush stdout");

                let mut cmd = String::new();
                io::stdin().read_line(&mut cmd).unwrap();

                let mut iter = cmd.trim_right().split_whitespace();
                match iter.next() {
                    Some("?") => self.do_help(),
                    Some("x") => break 'running,
                    Some("g") => {
                        event_pump.pump_events();
                        break_set = false
                    }
                    Some("l") => match iter.next() {
                        Some(f) => self.do_load(f),
                        None => self.do_dir(),
                    },
                    Some("s") | None => {
                        let mut step_cycles: u8 = 0;
                        let _ = self.do_single_step(&mut step_cycles);
                        println!(
                            "Stopped at PC: ${:04X} (cycles: {})",
                            self.cpu.get_pc(),
                            step_cycles
                        );
                        println!("{}", self.cpu)
                    }
                    Some("r") => println!("{}", self.cpu),
                    Some("m") => match iter.next() {
                        Some(a) => {
                            let mut addr = i64::from_str_radix(a.trim(), 16).unwrap() as usize;
                            for _ in 0..3 {
                                self.mem.memdump(addr, 16);
                                addr = addr + 16;
                            }
                        }
                        None => {}
                    },
                    Some("p") => match iter.next() {
                        Some(a) => {
                            let addr = i64::from_str_radix(a.trim(), 16).unwrap() as u16;
                            self.cpu.set_pc(addr)
                        }
                        None => {}
                    },
                    Some("b") => match iter.next() {
                        Some(a) => {
                            let addr = i64::from_str_radix(a.trim(), 16).unwrap() as u16;
                            self.cpu.set_breakpoint(addr)
                        }
                        None => {}
                    },
                    Some(_) => {}
                }
            }
        }
    }

    fn do_help(&self) {
        println!("Commands:");
        println!("(s)tep        - execute next instruction (single step)");
        println!("(g)o          - execute till next breakpoint");
        println!("(r)eg         - dump registers");
        println!("(m)em [addr]  - dump memory at addr");
        println!("(p)c [addr]   - set the PC to addr");
        println!("(b)p [addr]   - set breakpoint at addr");
        println!("(l)oad [file] - load a PRG file");
        println!("e(x)it        - exit program");
    }

    fn do_load(&mut self, filename: &str) {
        println!("Loading: {}", filename);
        if let Some(mut data) = load_file(&filename) {
            assert!(data.len() > 2);
            // We only load if the target is $0801 (start of BASIC)
            if (data[0] == 0x01) && (data[1] == 0x08) {
                let to_load: Vec<_> = data.drain(2..).collect();
                self.mem.load_ram(&to_load, 0x0801);

                // Fix up BASIC variable pointers
                let last: u16 = (0x0801 + to_load.len() + 1) as u16;
                let last_low = (last % 255) as u8;
                let last_high = (last / 255) as u8;
                self.mem.write_byte(0x002D, last_low);
                self.mem.write_byte(0x002E, last_high);
                self.mem.write_byte(0x002F, last_low);
                self.mem.write_byte(0x0030, last_high);
                println!("Load complete ({} bytes)", to_load.len());
            } else {
                println!("Unable to load - file does not target BASIC ($0801)");
                println!("Found target: ${:02X}{:02X}", data[1], data[0]);
            }
        }
    }

    fn do_dir(&self) {
        for path in fs::read_dir("./").unwrap() {
            println!("{}", path.unwrap().path().display())
        }
    }

    fn do_single_step(&mut self, cycles: &mut u8) -> bool {
        match self.cpu.single_step(&mut self.mem, cycles) {
            Ok(val) => val,
            Err(e) => {
                println!("Single step error: {}", e);
                true
            }
        }
    }
}
