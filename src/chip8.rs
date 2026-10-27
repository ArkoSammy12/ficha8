use rand::random;

const FONT_SIZE: usize = 5;
const FONT_DIGITS: usize = 16;
const FONT_BYTES: usize = FONT_SIZE * FONT_DIGITS;
const FONT_BEGIN_OFFSET: u16 = 0;

const PROGRAM_BEGIN: u16 = 0x200;

pub const RAM_SIZE: u16 = 4096;
pub const GFX_WIDTH: usize = 64;
pub const GFX_HEIGHT: usize = 32;
pub const GFX_LENGTH: usize = GFX_WIDTH * GFX_HEIGHT;
pub const IPF: usize = 15;
pub const FPS: u32 = 60;

const FONT: [u8; FONT_BYTES] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x60, 0x20, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0xA0, 0xA0, 0xF0, 0x20, 0x20, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x10, 0x10, 0x10, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xF0, 0x50, 0x70, 0x50, 0xF0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xF0, 0x50, 0x50, 0x50, 0xF0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
    ram: [u8; RAM_SIZE as usize],
    gfx: [bool; GFX_LENGTH],
    v: [u8; 16],
    stack: [u16; 16],
    keypad: [bool; 16],
    pc: u16,
    sp: u16,
    i: u16,
    dt: u8,
    st: u8,

    waiting_key: Option<u8>
}

enum CycleResult {
    None,
    Dxyn,
    Unknown,
}

impl Chip8 {

    pub fn new(rom: &[u8]) -> Result<Self, String> {
        if rom.len() > (RAM_SIZE - PROGRAM_BEGIN) as usize {
            return Err("ROM size too big!".to_string());
        }

        let mut chip8: Self = Self {
            ram: [0; _],
            gfx: [false; _],
            v: [0; _],
            stack: [0; _],
            keypad: [false; _],
            pc: PROGRAM_BEGIN,
            sp: 0,
            i: 0,
            dt: 0,
            st: 0,
            waiting_key: None
        };

        for i in 0..FONT_BYTES {
            chip8.ram[i + FONT_BEGIN_OFFSET as usize] = FONT[i];
        }

        for i in 0..rom.len() {
            chip8.ram[i + PROGRAM_BEGIN as usize] = rom[i];
        }

        Ok(chip8)
    }

    pub fn set_key(&mut self, index: usize, value: bool) {
        self.keypad[index & 0xF] = value;
    }

    pub fn play_buzzer(&self) -> bool {
        self.dt > 0
    }

    pub fn dump_argb8(&self) -> [u32; GFX_LENGTH] {
        let mut frame_buffer: [u32; GFX_LENGTH] = [0; _];
        for i in 0..GFX_LENGTH {
            frame_buffer[i] = if self.gfx[i] { u32::MAX } else { 0xFF000000 }
        }
        frame_buffer
    }

    pub fn execute_frame(&mut self) -> Result<(), String> {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }

        for _ in 0..IPF {
            let cycle_result: CycleResult = self.execute_cycle();
            match cycle_result {
                CycleResult::Dxyn => break,
                CycleResult::Unknown => return Err("Executed unknown opcode!".to_string()),
                CycleResult::None => {}
            }
        }

        Ok(())
    }
    fn execute_cycle(&mut self) -> CycleResult {
        let first: u8 = self.read(self.pc);
        let second: u8 = self.read(self.pc + 1);
        let opcode: u16 = ((first as u16) << 8) | second as u16;
        self.increment_pc();

        match first >> 4 {
            0x0 => {
                if first == 0x00 {
                    match second { // 00E0: cls
                        0xE0 => {
                            self.gfx.fill(false);
                        }
                        0xEE => { // 00EE: return
                            self.sp = (self.sp - 1) & 0xF;
                            self.pc = self.stack[self.sp as usize];
                        }
                        _ => {
                            return CycleResult::Unknown;
                        }
                    }
                } else {
                    return CycleResult::Unknown;
                }
            }
            0x1 => { // 1NNN: jump NNN
                self.pc = Self::get_nnn(opcode);
            }
            0x2 => { // 2NNN: :call NNN
                self.stack[self.sp as usize] = self.pc;
                self.sp = (self.sp + 1) & 0xF;
                self.pc = Self::get_nnn(opcode);
            }
            0x3 => { // 3XNN: if vX != NN then
                if Self::get_nn(opcode) == self.get_vx(opcode) {
                    self.increment_pc();
                }
            }
            0x4 => { // 4XNN: if vX == NN then
                if Self::get_nn(opcode) != self.get_vx(opcode) {
                    self.increment_pc();
                }
            }
            0x5 if Self::get_n(opcode) == 0x0 => { // 5XY0: if vX != vY then
                if self.get_vx(opcode) == self.get_vy(opcode) {
                    self.increment_pc()
                }
            }
            0x6 => { // 6XNN: vX := NN
                self.set_vx(opcode, Self::get_nn(opcode))
            }
            0x7 => { // 7XNN: vX += NN
                self.set_vx(opcode, Self::get_nn(opcode).wrapping_add(self.get_vx(opcode)))
            }
            0x8 => {
                match Self::get_n(opcode) {
                    0x0 => { // 8XY0: vX := vY
                        self.set_vx(opcode, self.get_vy(opcode))
                    }
                    0x1 => { // 8XY1: vX |= vY
                        self.v[Self::get_x(opcode) as usize] |= self.v[Self::get_y(opcode) as usize];
                        self.v[0xF] = 0
                    }
                    0x2 => { // 8XY2: vX &= vY
                        self.v[Self::get_x(opcode) as usize] &= self.v[Self::get_y(opcode) as usize];
                        self.v[0xF] = 0
                    }
                    0x3 => { // 8XY2: vX &= vY
                        self.v[Self::get_x(opcode) as usize] ^= self.v[Self::get_y(opcode) as usize];
                        self.v[0xF] = 0
                    }
                    0x4 => { // 8XY4: vX += vY
                        let vx: u8 = self.get_vx(opcode);
                        let vy: u8 = self.get_vy(opcode);
                        let (total, overflow) = vx.overflowing_add(vy);
                        self.set_vx(opcode, total);
                        self.v[0xF] = overflow as u8;
                    }
                    0x5 => { // 8XY5: vX -= vY
                        let vx: u8 = self.get_vx(opcode);
                        let vy: u8 = self.get_vy(opcode);
                        let (total, overflow) = vx.overflowing_sub(vy);
                        self.set_vx(opcode, total);
                        self.v[0xF] = !overflow as u8;
                    }
                    0x6 => { // 8XY6: vX >>= vY
                        let vy: u8 = self.get_vy(opcode);
                        self.set_vx(opcode, vy >> 1);
                        self.v[0xF] = vy & 1;
                    }
                    0x7 => { // 8XY7: vX =- vY
                        let vx: u8 = self.get_vx(opcode);
                        let vy: u8 = self.get_vy(opcode);
                        let (total, overflow) = vy.overflowing_sub(vx);
                        self.set_vx(opcode, total);
                        self.v[0xF] = !overflow as u8;
                    }
                    0xE => { // 8XYE: vX <<= vY
                        let vy: u8 = self.get_vy(opcode);
                        self.set_vx(opcode, vy << 1);
                        self.v[0xF] = (vy & 0x80 != 0) as u8;
                    }
                    _ => return CycleResult::Unknown
                }
            }
            0x9 if Self::get_n(opcode) == 0x0 => { // 9XY0: if vX == vY then
                if self.get_vx(opcode) != self.get_vy(opcode) {
                    self.increment_pc();
                }
            }
            0xA => { // ANNN: i := NNN
                self.i = Self::get_nnn(opcode);
            }
            0xB => { // BNNN: jump0 NNN
                self.pc = (Self::get_nnn(opcode).wrapping_add(self.v[0] as u16)) & 0xFFF;
            }
            0xC => { // CXNN: vX := random NN
                self.set_vx(opcode, random::<u8>() & Self::get_nn(opcode))
            }
            0xD => { // DXYN: sprite vX vY N
                let sprite_x: u8 = self.get_vx(opcode) % (GFX_WIDTH as u8);
                let sprite_y: u8 = self.get_vy(opcode) % (GFX_HEIGHT as u8);
                let sprite_height: u8 = Self::get_n(opcode);
                self.v[0xF] = 0;
                for i in 0..sprite_height {
                    let slice_y: u8 = sprite_y + i;
                    if slice_y >= (GFX_HEIGHT as u8) {
                        break;
                    }
                    let slice: u8 = self.read(self.i.wrapping_add(i as u16));
                    let mut slice_mask: u8 = 0x80;
                    for j in 0..8 {
                        let slice_x: u8 = sprite_x + j;
                        if slice_x >= (GFX_WIDTH as u8) {
                            break;
                        }
                        if slice & slice_mask != 0 {
                            let index: usize = (((slice_y as usize).wrapping_mul(GFX_WIDTH)) + (slice_x as usize));
                            self.gfx[index] = !self.gfx[index];
                            if !self.gfx[index] {
                                self.v[0xF] = 1;
                            }
                        }
                         slice_mask >>= 1;
                    }
                }
                return CycleResult::Dxyn
            }
            0xE => {
                match Self::get_nn(opcode) {
                    0x9E => { // EX9E: if vX -key then
                        if self.keypad[(self.get_vx(opcode) as usize) & 0xF] {
                            self.increment_pc();
                        }
                    }
                    0xA1 => { // EXA1: if vX key then
                        if !self.keypad[(self.get_vx(opcode) as usize) & 0xF] {
                            self.increment_pc();
                        }
                    }
                    _ => return CycleResult::None
                }
            }
            0xF => {
                match Self::get_nn(opcode) {
                    0x07 => { // FX07: vX := delay
                        self.set_vx(opcode, self.dt)
                    }
                    0x0A => { // FX0A: vX := key
                        let mut first_pressed_key: Option<u8> = None;
                        for i in 0..self.keypad.len() {
                            if self.keypad[i] {
                                first_pressed_key = Some(i as u8);
                                break;
                            }
                        }
                        if let Some(waiting_key) = self.waiting_key {
                            if first_pressed_key.is_none() || self.waiting_key != first_pressed_key {
                                self.set_vx(opcode, waiting_key & 0xF);
                                self.waiting_key = None;
                            }
                        } else {
                            self.waiting_key = first_pressed_key;
                            self.decrement_pc();
                        }
                    }
                    0x15 => { // FX15: delay := vX
                        self.dt = self.get_vx(opcode);
                    }
                    0x18 => { // FX18: buzzer := vX
                        self.st = self.get_vx(opcode);
                    }
                    0x1E => { // FX1E: i += vX
                        self.i = self.i.wrapping_add(self.get_vx(opcode) as u16) & 0xFFF
                    }
                    0x29 => { // FX29: i := hex vX
                        self.i = FONT_BEGIN_OFFSET + (FONT_SIZE as u16) * ((self.get_vx(opcode) as u16) & 0xF)
                    }
                    0x33 => { // FXEE: bcd vX
                        let i: u16 = self.i;
                        let vx: u8 = self.get_vx(opcode);
                        let hundreds: u64 = ((vx as u64) * 0x51EB851Fu64) >> 37;
                        let remainder: u64 = (vx as u64) - hundreds * 100;
                        let tens: u64 = (remainder * 0xCCCDu64) >> 19;
                        let ones: u64 = remainder - tens * 10;
                        self.write(i, hundreds as u8);
                        self.write(i.wrapping_add(1), tens as u8);
                        self.write(i.wrapping_add(2), ones as u8);
                    }
                    0x55 => { // FX55: save vX
                        let x: u8 = Self::get_x(opcode);
                        for i in 0..=x {
                            self.write(self.i.wrapping_add(i as u16), self.v[i as usize]);
                        }
                        self.i = self.i.wrapping_add((x as u16) + 1)& 0xFFF;
                    }
                    0x65 => { // FX65: load vX
                        let x: u8 = Self::get_x(opcode);
                        for i in 0..=x {
                            self.v[i as usize] = self.read(self.i.wrapping_add(i as u16));
                        }
                        self.i = self.i.wrapping_add((x as u16) + 1)& 0xFFF;
                    }
                    _ => return CycleResult::Unknown
                }
            }
            _ => return CycleResult::Unknown
        }
        CycleResult::None
    }

    fn increment_pc(&mut self) {
        self.pc += 2;
    }

    fn decrement_pc(&mut self) {
        self.pc -= 2;
    }

    fn read(&mut self, address: u16) -> u8 {
        self.ram[(address & 0xFFF) as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.ram[(address & 0xFFF) as usize] =  value;
    }

    fn get_nnn(opcode: u16) -> u16 {
        opcode & 0xFFF
    }

    fn get_nn(opcode: u16) -> u8 {
        opcode as u8
    }

    fn get_n(opcode: u16) -> u8 {
        (opcode as u8) & 0xF
    }

    fn get_x(opcode: u16) -> u8 {
        ((opcode & 0x0F00) >> 8) as u8
    }

    fn get_y(opcode: u16) -> u8 {
        (opcode & 0x00F0) as u8 >> 4
    }

    fn get_vx(&self, opcode: u16) -> u8 {
        self.v[Self::get_x(opcode) as usize]
    }

    fn set_vx(&mut self, opcode: u16, value: u8) {
        self.v[Self::get_x(opcode) as usize] = value;
    }

    fn get_vy(&self, opcode: u16) -> u8 {
        self.v[Self::get_y(opcode) as usize]
    }

}