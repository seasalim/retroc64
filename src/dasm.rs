use memory::*;
use opcodes::*;

pub fn disassemble_step(mem: &MemIO, mut ip: &mut u16) -> Result<String, String> {
    let mut result = String::new();

    let s = try!(parse_opcode(mem, &mut ip));
    result.push_str(&s);

    Ok(result)
}

pub fn parse_opcode(mem: &MemIO, ip: &mut u16) -> Result<String, String> {
    let b = mem.read_byte(*ip as u16);
    let opcode = match get_opcode(b) {
        Some(op) => op,
        None => {
            return Err(format!(
                "Invalid opcode {:02X} (IP = {:04X})!",
                b, *ip as usize
            ))
        }
    };

    let addr = *ip;
    *ip += 1;

    let s = match opcode.mode {
        Mode::Implied | Mode::Accumulator => "".to_string(),
        Mode::Immediate => {
            *ip += 1;
            format!(" #${:02X}", mem.read_byte(*ip as u16 - 1))
        }
        Mode::Absolute => {
            *ip += 2;
            format!(
                " ${:02X}{:02X}",
                mem.read_byte(*ip as u16 - 1),
                mem.read_byte(*ip as u16 - 2)
            )
        }
        Mode::AbsoluteX => {
            *ip += 2;
            format!(
                " ${:02X}{:02X}, X",
                mem.read_byte(*ip as u16 - 1),
                mem.read_byte(*ip as u16 - 2)
            )
        }
        Mode::AbsoluteY => {
            *ip += 2;
            format!(
                " ${:02X}{:02X}, Y",
                mem.read_byte(*ip as u16 - 1),
                mem.read_byte(*ip as u16 - 2)
            )
        }
        Mode::Indirect => {
            *ip += 2;
            format!(
                " (${:02X}{:02X})",
                mem.read_byte(*ip as u16 - 1),
                mem.read_byte(*ip as u16 - 2)
            )
        }
        Mode::IndirectX => {
            *ip += 1;
            format!(" (${:02X}, X)", mem.read_byte(*ip as u16 - 1))
        }
        Mode::IndirectY => {
            *ip += 1;
            format!(" (${:02X}), Y", mem.read_byte(*ip as u16 - 1))
        }
        Mode::ZeroPage => {
            *ip += 1;
            format!(" ${:02X}", mem.read_byte(*ip as u16 - 1))
        }
        Mode::ZeroPageX => {
            *ip += 1;
            format!(" ${:02X}, X", mem.read_byte(*ip as u16 - 1))
        }
        Mode::ZeroPageY => {
            *ip += 1;
            format!(" ${:02X}, Y", mem.read_byte(*ip as u16 - 1))
        }
        Mode::Relative => {
            *ip += 1;
            format!(" ${:02X}", mem.read_byte(*ip as u16 - 1))
        }
    };

    let result = format!("${:04X} {}{}", addr, opcode.name, s);
    Ok(result)
}
