#[macro_use]
extern crate log;
extern crate clap;
extern crate env_logger;
use clap::{App, Arg};
extern crate sdl2;
extern crate time;

mod cpu;
mod dasm;
mod emu;
mod keyboard;
mod memc64;
mod memory;
mod opcodes;
mod utils;
mod vic;

fn main() {
    env_logger::init();

    let matches = App::new("retroc64")
        .version("0.1.0")
        .author("Salim Alam")
        .about("Commodore 64 Emulator")
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("Debug the emulator in monitor mode"),
        )
        .arg(
            Arg::with_name("start_addr")
                .short("s")
                .long("start_addr")
                .value_name("ADDRESS")
                .help("Start address for Program Counter (hexadecimal) (default: FCE2)")
                .takes_value(true),
        )
        .get_matches();

    let s_addr = matches.value_of("start_addr").unwrap_or("FCE2");
    let start_addr = i64::from_str_radix(s_addr.trim(), 16).unwrap() as u16;
    let debug = matches.is_present("debug");

    let mut c64 = emu::C64::new();
    c64.start(start_addr, debug);
}
