pub trait MemIO {
    fn read_byte(&self, addr: u16) -> u8;
    fn read_word(&self, addr: u16) -> u16;
    fn write_byte(&mut self, addr: u16, val: u8);
    fn memdump(&self, from: usize, bytes: usize);
}
