use memory::*;
use opcodes::*;
use std::fmt;

pub struct CPU {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    st: u8,
    sp: u8,
    brk: bool,
    brkpt: u16,
}

pub enum Flags {
    Carry = 0b00000001,
    Zero = 0b00000010,
    Interrupt = 0b00000100,
    Decimal = 0b00001000,
    Break = 0b00010000,
    Overflow = 0b01000000,
    Sign = 0b10000000,
}

pub enum Operand {
    Accumulator,
    Value(u8),
    Address(u16),
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0,
            a: 0,
            x: 0,
            y: 0,
            st: 0x20,
            sp: 0xFF,
            brk: false,
            brkpt: 0xFFFF,
        }
    }

    pub fn get_pc(&mut self) -> u16 {
        self.pc
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.pc = pc;
    }

    pub fn get_flag(&self, f: Flags) -> bool {
        (self.st & f as u8) > 0
    }

    pub fn set_flag(&mut self, f: Flags, v: bool) {
        match v {
            true => self.st |= f as u8,
            false => self.st &= !(f as u8),
        }
    }

    pub fn set_breakpoint(&mut self, addr: u16) {
        self.brkpt = addr;
    }

    pub fn trigger_irq(&mut self, mem: &mut MemIO) {
        self.do_irq(mem)
    }

    pub fn single_step(&mut self, mem: &mut MemIO, cycles: &mut u8) -> Result<bool, String> {
        let b = mem.read_byte(self.pc);
        let opcode = match get_opcode(b) {
            Some(op) => op,
            None => return Err(format!("Invalid opcode {:02X} (IP = {:04X})!", b, self.pc)),
        };

        let opr: Operand = self.fetch_operand(mem, opcode);
        let val: u8 = self.operand_source(mem, &opr);
        let addr: u16 = self.operand_target(&opr);

        // println!("{:04X} {}", self.pc, opcode.name);

        self.pc += opcode.bytes as u16;
        *cycles = opcode.cycles;

        match opcode.name {
            "ADC" => self.do_adc(val),
            "AND" => self.do_and(val),
            "ASL" => self.do_asl(mem, opr),
            "BCC" => self.do_bcc(val),
            "BCS" => self.do_bcs(val),
            "BEQ" => self.do_beq(val),
            "BIT" => self.do_bit(val),
            "BRK" => self.do_brk(mem),
            "BMI" => self.do_bmi(val),
            "BNE" => self.do_bne(val),
            "BPL" => self.do_bpl(val),
            "BVC" => self.do_bvc(val),
            "BVS" => self.do_bvs(val),
            "CLC" => self.do_clc(),
            "CLD" => self.do_cld(),
            "CLI" => self.do_cli(),
            "CLV" => self.do_clv(),
            "CMP" => self.do_cmp(val),
            "CPX" => self.do_cpx(val),
            "CPY" => self.do_cpy(val),
            "DEC" => self.do_dec(mem, addr),
            "DEX" => self.do_dex(),
            "DEY" => self.do_dey(),
            "EOR" => self.do_eor(val),
            "INC" => self.do_inc(mem, addr),
            "INX" => self.do_inx(),
            "INY" => self.do_iny(),
            "JMP" => self.do_jmp(addr),
            "JSR" => self.do_jsr(mem, addr),
            "LDA" => self.do_lda(val),
            "LDX" => self.do_ldx(val),
            "LDY" => self.do_ldy(val),
            "LSR" => self.do_lsr(mem, opr),
            "NOP" => self.do_nop(),
            "ORA" => self.do_ora(val),
            "PHA" => self.do_pha(mem),
            "PHP" => self.do_php(mem),
            "PLA" => self.do_pla(mem),
            "PLP" => self.do_plp(mem),
            "ROL" => self.do_rol(mem, opr),
            "ROR" => self.do_ror(mem, opr),
            "RTI" => self.do_rti(mem),
            "RTS" => self.do_rts(mem),
            "SBC" => self.do_sbc(val),
            "SEC" => self.do_sec(),
            "SED" => self.do_sed(),
            "SEI" => self.do_sei(),
            "STA" => self.do_sta(mem, addr),
            "STX" => self.do_stx(mem, addr),
            "STY" => self.do_sty(mem, addr),
            "TAX" => self.do_tax(),
            "TAY" => self.do_tay(),
            "TSX" => self.do_tsx(),
            "TXA" => self.do_txa(),
            "TYA" => self.do_tya(),
            "TXS" => self.do_txs(),
            _ => {
                return Err(format!(
                    "Unhandled opcode {:02X} {}",
                    opcode.code, opcode.name
                ))
            }
        };

        let dobreak = self.brk;
        self.brk = false;

        Ok(dobreak || (self.brkpt == self.pc))
    }

    fn fetch_operand(&self, mem: &mut MemIO, opcode: &Opcode) -> Operand {
        match opcode.mode {
            Mode::Immediate => self.opr_immediate(mem),
            Mode::Absolute => self.opr_absolute(mem),
            Mode::AbsoluteX => self.opr_absolute_x(mem),
            Mode::AbsoluteY => self.opr_absolute_y(mem),
            Mode::ZeroPage => self.opr_zeropage(mem),
            Mode::ZeroPageX => self.opr_zeropage_x(mem),
            Mode::ZeroPageY => self.opr_zeropage_y(mem),
            Mode::Indirect => self.opr_indirect(mem),
            Mode::IndirectX => self.opr_indirect_x(mem),
            Mode::IndirectY => self.opr_indirect_y(mem),
            Mode::Relative => self.opr_relative(mem),
            Mode::Accumulator => self.opr_accumulator(),
            Mode::Implied => Operand::Value(0),
        }
    }

    fn opr_accumulator(&self) -> Operand {
        Operand::Accumulator
    }

    fn opr_immediate(&self, mem: &mut MemIO) -> Operand {
        Operand::Value(mem.read_byte(self.pc + 1))
    }

    fn opr_absolute(&self, mem: &mut MemIO) -> Operand {
        Operand::Address(mem.read_word(self.pc + 1))
    }

    fn opr_absolute_x(&self, mem: &mut MemIO) -> Operand {
        Operand::Address(self.x as u16 + mem.read_word(self.pc + 1))
    }

    fn opr_absolute_y(&self, mem: &mut MemIO) -> Operand {
        Operand::Address(self.y as u16 + mem.read_word(self.pc + 1))
    }

    fn opr_zeropage(&self, mem: &mut MemIO) -> Operand {
        Operand::Address((mem.read_byte(self.pc + 1) as u16) & 0xFF)
    }

    fn opr_zeropage_x(&self, mem: &mut MemIO) -> Operand {
        Operand::Address((self.x as u16 + mem.read_byte(self.pc + 1) as u16) & 0xFF)
    }

    fn opr_zeropage_y(&self, mem: &mut MemIO) -> Operand {
        Operand::Address((self.y as u16 + mem.read_byte(self.pc + 1) as u16) & 0xFF)
    }

    fn opr_indirect(&self, mem: &mut MemIO) -> Operand {
        Operand::Address(mem.read_word(mem.read_word(self.pc + 1) as u16))
    }

    fn opr_indirect_x(&self, mem: &mut MemIO) -> Operand {
        Operand::Address(mem.read_word((self.x as u16 + mem.read_byte(self.pc + 1) as u16) & 0xFF))
    }

    fn opr_indirect_y(&self, mem: &mut MemIO) -> Operand {
        Operand::Address(self.y as u16 + mem.read_word(mem.read_byte(self.pc + 1) as u16))
    }

    fn opr_relative(&self, mem: &mut MemIO) -> Operand {
        Operand::Value(mem.read_byte(self.pc + 1))
    }

    fn push_byte(&mut self, mem: &mut MemIO, val: u8) {
        mem.write_byte(0x100 + (self.sp as u16), val);
        self.sp = (((self.sp as i16) - 1) & 0xFF) as u8;
    }

    fn pop_byte(&mut self, mem: &mut MemIO) -> u8 {
        self.sp = (((self.sp as u16) + 1) & 0xFF) as u8;
        mem.read_byte(0x100 + (self.sp as u16))
    }

    fn push_word(&mut self, mem: &mut MemIO, addr: u16) {
        let lobyte = addr & 0xFF;
        let hibyte = (addr & 0xFF00) >> 8;
        self.push_byte(mem, hibyte as u8);
        self.push_byte(mem, lobyte as u8)
    }

    fn pop_word(&mut self, mem: &mut MemIO) -> u16 {
        let lobyte = self.pop_byte(mem);
        let hibyte = self.pop_byte(mem);
        ((hibyte as u16) << 8) | (lobyte as u16)
    }

    fn operand_source(&self, mem: &mut MemIO, operand: &Operand) -> u8 {
        match *operand {
            Operand::Accumulator => self.a,
            Operand::Value(v) => v,
            Operand::Address(a) => mem.read_byte(a),
        }
    }

    fn operand_target(&self, operand: &Operand) -> u16 {
        match *operand {
            Operand::Accumulator => self.a as u16,
            Operand::Value(v) => v as u16,
            Operand::Address(a) => a,
        }
    }

    fn set_zero_sign_for(&mut self, val: u8) {
        self.set_flag(Flags::Zero, val == 0);
        self.set_flag(Flags::Sign, val > 127)
    }

    fn jmp_relative(&mut self, offset: u8) {
        if offset > 127 {
            self.pc = self.pc - (256 - offset as u16);
        } else {
            self.pc = self.pc + offset as u16;
        }
    }

    fn do_adc(&mut self, val: u8) {
        let carry: u8 = if self.get_flag(Flags::Carry) { 1 } else { 0 };

        let mut result = self.a as u16 + val as u16 + carry as u16;
        let &acc = &self.a;

        if self.get_flag(Flags::Decimal) {
            self.set_flag(Flags::Zero, (result as u8) == 0);
            if ((acc & 0xF) + (val & 0xF) + carry) > 9 {
                result += 6;
            };
            self.set_flag(Flags::Sign, (result as u8) > 127);
            self.set_flag(
                Flags::Overflow,
                (((acc ^ val) & 0x80) == 0) && (((acc ^ (result as u8)) & 0x80) > 0),
            );

            if (result as u16) > 0x99 {
                result += 96;
            };
            self.set_flag(Flags::Carry, (result as u16) > 0x99)
        } else {
            self.set_zero_sign_for(result as u8);
            self.set_flag(Flags::Carry, result > 0xFF);

            self.set_flag(
                Flags::Overflow,
                (((acc ^ val) & 0x80) == 0) && (((acc ^ (result as u8)) & 0x80) > 0),
            )
        }

        self.a = result as u8
    }

    fn do_and(&mut self, val: u8) {
        let result = self.a & val;
        self.set_zero_sign_for(result);
        self.a = result;
    }

    fn do_asl(&mut self, mem: &mut MemIO, opr: Operand) {
        let mut val = self.operand_source(mem, &opr);
        self.set_flag(Flags::Carry, (val & 0x80) > 0);
        val = val << 1;
        self.set_zero_sign_for(val);

        match opr {
            Operand::Accumulator => self.a = val,
            _ => {
                let addr = self.operand_target(&opr);
                mem.write_byte(addr, val)
            }
        }
    }

    fn do_bcc(&mut self, offset: u8) {
        if !self.get_flag(Flags::Carry) {
            self.jmp_relative(offset)
        }
    }

    fn do_bcs(&mut self, offset: u8) {
        if self.get_flag(Flags::Carry) {
            self.jmp_relative(offset)
        }
    }

    fn do_beq(&mut self, offset: u8) {
        if self.get_flag(Flags::Zero) {
            self.jmp_relative(offset)
        }
    }

    fn do_bit(&mut self, val: u8) {
        self.set_flag(Flags::Sign, val > 127);
        self.set_flag(Flags::Overflow, (val & 0x40) > 0);

        let result = self.a & val;
        self.set_flag(Flags::Zero, result == 0);
    }

    fn do_bmi(&mut self, offset: u8) {
        if self.get_flag(Flags::Sign) {
            self.jmp_relative(offset)
        }
    }

    fn do_bne(&mut self, offset: u8) {
        if !self.get_flag(Flags::Zero) {
            self.jmp_relative(offset)
        }
    }

    fn do_bpl(&mut self, offset: u8) {
        if !self.get_flag(Flags::Sign) {
            self.jmp_relative(offset)
        }
    }

    fn do_bvc(&mut self, offset: u8) {
        if !self.get_flag(Flags::Overflow) {
            self.jmp_relative(offset)
        }
    }

    fn do_bvs(&mut self, offset: u8) {
        if self.get_flag(Flags::Overflow) {
            self.jmp_relative(offset)
        }
    }

    fn do_brk(&mut self, mem: &mut MemIO) {
        // self.brk = true;
        self.pc = self.pc + 1;
        let retaddr = self.pc;
        self.push_word(mem, retaddr);
        self.set_flag(Flags::Break, true);
        let status = self.st;
        self.push_byte(mem, status);
        self.set_flag(Flags::Interrupt, true);
        self.pc = mem.read_word(0xFFFE)
    }

    fn do_irq(&mut self, mem: &mut MemIO) {
        if !self.get_flag(Flags::Interrupt) {
            let retaddr = self.pc;
            self.push_word(mem, retaddr);
            self.set_flag(Flags::Break, false);
            let status = self.st;
            self.push_byte(mem, status);
            self.set_flag(Flags::Interrupt, true);
            self.pc = mem.read_word(0xFFFE);
        }
    }

    fn do_clc(&mut self) {
        self.set_flag(Flags::Carry, false)
    }

    fn do_cld(&mut self) {
        self.set_flag(Flags::Decimal, false)
    }

    fn do_cli(&mut self) {
        self.set_flag(Flags::Interrupt, false)
    }

    fn do_clv(&mut self) {
        self.set_flag(Flags::Overflow, false)
    }

    fn do_cmp(&mut self, val: u8) {
        let result: i16 = self.a as i16 - val as i16;
        self.set_flag(Flags::Carry, (result as u16) < 0x100);
        self.set_zero_sign_for(result as u8)
    }

    fn do_cpx(&mut self, val: u8) {
        let result: i16 = self.x as i16 - val as i16;
        self.set_flag(Flags::Carry, (result as u16) < 0x100);
        self.set_zero_sign_for(result as u8);
    }

    fn do_cpy(&mut self, val: u8) {
        let result: i16 = self.y as i16 - val as i16;
        self.set_flag(Flags::Carry, (result as u16) < 0x100);
        self.set_zero_sign_for(result as u8);
    }

    fn do_dec(&mut self, mem: &mut MemIO, addr: u16) {
        let val = (mem.read_byte(addr) as i16 - 1) as u8;
        self.set_zero_sign_for(val);
        mem.write_byte(addr, val)
    }

    fn do_dex(&mut self) {
        let result: i16 = self.x as i16 - 1;
        self.x = result as u8;
        self.set_zero_sign_for(result as u8)
    }

    fn do_dey(&mut self) {
        let result: i16 = self.y as i16 - 1;
        self.y = result as u8;
        self.set_zero_sign_for(result as u8)
    }

    fn do_eor(&mut self, val: u8) {
        let result = self.a ^ val;
        self.set_zero_sign_for(result);
        self.a = result;
    }

    fn do_inc(&mut self, mem: &mut MemIO, addr: u16) {
        let val = (mem.read_byte(addr) as u16 + 1) as u8;
        self.set_zero_sign_for(val);
        mem.write_byte(addr, val)
    }

    fn do_inx(&mut self) {
        let val = ((self.x as u16) + 1) & 0xFF;
        self.set_zero_sign_for(val as u8);
        self.x = val as u8;
    }

    fn do_iny(&mut self) {
        let val = ((self.y as u16) + 1) & 0xFF;
        self.set_zero_sign_for(val as u8);
        self.y = val as u8
    }

    fn do_jmp(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn do_jsr(&mut self, mem: &mut MemIO, addr: u16) {
        let val = self.pc - 1;
        self.push_word(mem, val);
        self.pc = addr
    }

    fn do_lda(&mut self, val: u8) {
        self.a = val;
        self.set_zero_sign_for(val)
    }

    fn do_ldx(&mut self, val: u8) {
        self.x = val;
        self.set_zero_sign_for(val)
    }

    fn do_ldy(&mut self, val: u8) {
        self.y = val;
        self.set_zero_sign_for(val)
    }

    fn do_lsr(&mut self, mem: &mut MemIO, opr: Operand) {
        let mut val = self.operand_source(mem, &opr);
        self.set_flag(Flags::Carry, (val & 0x01) > 0);
        val = val >> 1;
        self.set_zero_sign_for(val);

        match opr {
            Operand::Accumulator => self.a = val,
            _ => {
                let addr = self.operand_target(&opr);
                mem.write_byte(addr, val)
            }
        }
    }

    fn do_ora(&mut self, val: u8) {
        let result = self.a | val;
        self.set_zero_sign_for(result);
        self.a = result;
    }

    fn do_rol(&mut self, mem: &mut MemIO, opr: Operand) {
        let mut val = self.operand_source(mem, &opr) as u16;
        val = val << 1;
        if self.get_flag(Flags::Carry) {
            val = val | 0x01;
        }
        self.set_flag(Flags::Carry, val > 0xFF);
        self.set_zero_sign_for((val & 0xFF) as u8);

        match opr {
            Operand::Accumulator => self.a = val as u8,
            _ => {
                let addr = self.operand_target(&opr);
                mem.write_byte(addr, val as u8)
            }
        }
    }

    fn do_ror(&mut self, mem: &mut MemIO, opr: Operand) {
        let mut val = self.operand_source(mem, &opr) as u16;
        if self.get_flag(Flags::Carry) {
            val = val | 0x100;
        }
        self.set_flag(Flags::Carry, (val & 0x01) > 0);
        val = val >> 1;
        self.set_zero_sign_for((val & 0xFF) as u8);

        match opr {
            Operand::Accumulator => self.a = val as u8,
            _ => {
                let addr = self.operand_target(&opr);
                mem.write_byte(addr, val as u8)
            }
        }
    }

    fn do_nop(&mut self) {}

    fn do_pha(&mut self, mem: &mut MemIO) {
        let val = self.a;
        self.push_byte(mem, val)
    }

    fn do_pla(&mut self, mem: &mut MemIO) {
        let val = self.pop_byte(mem);
        self.a = val;
        self.set_zero_sign_for(val)
    }

    fn do_plp(&mut self, mem: &mut MemIO) {
        self.st = self.pop_byte(mem) | 0x20;
    }

    fn do_php(&mut self, mem: &mut MemIO) {
        let val = self.st | 0x30;
        self.push_byte(mem, val)
    }

    fn do_rti(&mut self, mem: &mut MemIO) {
        self.st = self.pop_byte(mem) | 0x20;
        self.pc = self.pop_word(mem);
    }

    fn do_rts(&mut self, mem: &mut MemIO) {
        let addr = self.pop_word(mem) + 1;
        self.pc = addr
    }

    fn do_sbc(&mut self, val: u8) {
        let carry: u8 = if self.get_flag(Flags::Carry) { 0 } else { 1 };

        let mut result = self.a as i16 - val as i16 - carry as i16;
        self.set_zero_sign_for(result as u8);

        let &acc = &self.a;
        self.set_flag(
            Flags::Overflow,
            (((acc ^ (result as u8)) & 0x80) > 0) && (((acc ^ val) & 0x80) > 0),
        );

        if self.get_flag(Flags::Decimal) {
            if (((acc as i16) & 0xF) - carry as i16) < ((val as i16) & 0xF) {
                result -= 6;
            }
            if (result as u16) > 0x99 {
                result -= 0x60;
            }
        }

        self.set_flag(Flags::Carry, (result as u16) < 0x100);
        self.a = result as u8
    }

    fn do_sec(&mut self) {
        self.set_flag(Flags::Carry, true)
    }

    fn do_sed(&mut self) {
        self.set_flag(Flags::Decimal, true)
    }

    fn do_sei(&mut self) {
        self.set_flag(Flags::Interrupt, true)
    }

    fn do_sta(&mut self, mem: &mut MemIO, addr: u16) {
        let val = self.a;
        mem.write_byte(addr, val)
    }

    fn do_stx(&mut self, mem: &mut MemIO, addr: u16) {
        let val = self.x;
        mem.write_byte(addr, val)
    }

    fn do_sty(&mut self, mem: &mut MemIO, addr: u16) {
        let val = self.y;
        mem.write_byte(addr, val)
    }

    fn do_tax(&mut self) {
        let val = self.a;
        self.x = self.a;
        self.set_zero_sign_for(val)
    }

    fn do_tay(&mut self) {
        let val = self.a;
        self.y = self.a;
        self.set_zero_sign_for(val)
    }

    fn do_tsx(&mut self) {
        let val = self.sp;
        self.x = self.sp;
        self.set_zero_sign_for(val)
    }

    fn do_txa(&mut self) {
        let val = self.x;
        self.a = self.x;
        self.set_zero_sign_for(val)
    }

    fn do_tya(&mut self) {
        let val = self.y;
        self.a = self.y;
        self.set_zero_sign_for(val)
    }

    fn do_txs(&mut self) {
        self.sp = self.x
    }

    fn pp_flags(&self, flags: &u8) -> String {
        let mut v = vec!['S', 'V', '-', 'B', 'D', 'I', 'Z', 'C'];
        for ix in 0..8 {
            if (flags & (1 << ix)) == 0 {
                v[7 - ix] = '.';
            }
        }
        v.into_iter().collect()
    }
}

impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PC:${:04X} A:${:02X} X:${:02X} Y:${:02X} SP:${:02X} ST:{:08b} [{}]",
            self.pc,
            self.a,
            self.x,
            self.y,
            self.sp,
            self.st,
            self.pp_flags(&self.st)
        )
    }
}
