use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn hexdump(buf: &Vec<u8>, from: usize, bytes: usize) {
    // Print offset
    print!("{:04X}: ", from);

    // Print hex bytes
    for n in from..from + bytes {
        match buf.get(n) {
            Some(x) => print!("{:02X} ", x),
            None => print!("   "),
        }
    }

    // Print ascii bytes
    print!(" ");
    for n in from..from + bytes {
        match buf.get(n) {
            Some(x) => {
                if (*x as char).is_alphanumeric() {
                    print!("{}", *x as char);
                } else {
                    print!(".");
                }
            }
            None => print!("."),
        }
    }

    println!("");
}

pub fn load_file(filename: &str) -> Option<Vec<u8>> {
    let path = Path::new(&filename);
    let mut file = match File::open(&path) {
        Err(err) => {
            println!("Couldn't open {}: {:?}", filename, err);
            return None;
        }
        Ok(file) => file,
    };

    let mut buf = Vec::new();
    match file.read_to_end(&mut buf) {
        Err(err) => {
            println!("Couldn't read {}: {:?}", filename, err);
            return None;
        }
        Ok(_) => {}
    };

    Some(buf)
}
