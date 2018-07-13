use sdl2::keyboard::Scancode;

// Keyboard scan codes
static KBD_ROWS: &'static [[Scancode; 8]; 8] = &[
    // Row 0 = $FE
    [
        Scancode::Backspace,
        Scancode::Return,
        Scancode::Right,
        Scancode::F7,
        Scancode::F1,
        Scancode::F3,
        Scancode::F5,
        Scancode::Down,
    ],
    // Row 1 = $FD
    [
        Scancode::Num3,
        Scancode::W,
        Scancode::A,
        Scancode::Num4,
        Scancode::Z,
        Scancode::S,
        Scancode::E,
        Scancode::LShift,
    ],
    // Row 2 = $FB
    [
        Scancode::Num5,
        Scancode::R,
        Scancode::D,
        Scancode::Num6,
        Scancode::C,
        Scancode::F,
        Scancode::T,
        Scancode::X,
    ],
    // Row 3 = $F7
    [
        Scancode::Num7,
        Scancode::Y,
        Scancode::G,
        Scancode::Num8,
        Scancode::B,
        Scancode::H,
        Scancode::U,
        Scancode::V,
    ],
    // Row 4 = $EF
    [
        Scancode::Num9,
        Scancode::I,
        Scancode::J,
        Scancode::Num0,
        Scancode::M,
        Scancode::K,
        Scancode::O,
        Scancode::N,
    ],
    // Row 5 = $DF
    [
        Scancode::Minus,
        Scancode::P,
        Scancode::L,
        Scancode::Equals,
        Scancode::Period,
        Scancode::Semicolon,   // Colon
        Scancode::LeftBracket, // At Symbol
        Scancode::Comma,
    ],
    // Row 6 = $BF
    [
        Scancode::Insert,       // Pound
        Scancode::RightBracket, // Asterisk
        Scancode::Apostrophe,   // Semicolon
        Scancode::Home,
        Scancode::RShift,
        Scancode::Backslash, // Equals
        Scancode::Delete,    // ^
        Scancode::Slash,
    ],
    // Row 7 = $7F
    [
        Scancode::Num1,
        Scancode::Grave, // <
        Scancode::Tab,   // Ctrl
        Scancode::Num2,
        Scancode::Space,
        Scancode::LCtrl,
        Scancode::Q,
        Scancode::Pause,
    ],
];

pub fn kbd_scancode(keys: &Vec<Scancode>, col_mask: u8) -> u8 {
    if keys.is_empty() {
        return 0xFF; // No key pressed
    }

    //let col_mask = !self.io[0xDC00 - 0xD000];
    let mut val = 0;
    for sc in keys.iter() {
        for i in 0..8 {
            if (col_mask & (1 << i)) > 0 {
                match KBD_ROWS[i].iter().position(|&x| x == *sc) {
                    Some(n) => val = val | (1 << n),
                    None => (),
                }
            }
        }
    }

    !val
}
