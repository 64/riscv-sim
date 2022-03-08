use crate::util::Addr;

#[derive(Debug, Clone)]
pub struct Memory {
    mem: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { mem: vec![0; 120] }
    }

    pub fn readb(&self, addr: Addr) -> u32 {
        self.mem[addr.0 as usize] as u32
    }

    pub fn readh(&self, addr: Addr) -> u32 {
        let a = addr.0 as usize;
        assert!(a % 2 == 0);

        u16::from_le_bytes([self.mem[a], self.mem[a + 1]]) as u32
    }

    pub fn readw(&self, addr: Addr) -> u32 {
        let a = addr.0 as usize;
        assert!(a % 4 == 0);

        u32::from_le_bytes([
            self.mem[a],
            self.mem[a + 1],
            self.mem[a + 2],
            self.mem[a + 3],
        ])
    }

    pub fn writeb(&mut self, addr: Addr, val: u32) {
        self.mem[addr.0 as usize] = val.to_le_bytes()[0];
    }

    pub fn writeh(&mut self, addr: Addr, val: u32) {
        let a = addr.0 as usize;
        assert!(a % 2 == 0);

        self.mem[a..a + 2].copy_from_slice(&val.to_le_bytes())
    }

    pub fn writew(&mut self, addr: Addr, val: u32) {
        let a = addr.0 as usize;
        assert!(a % 4 == 0);

        self.mem[a..a + 4].copy_from_slice(&val.to_le_bytes())
    }
}
