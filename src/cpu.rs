use crate::{
    inst::{ArchReg, Inst, MemRef},
    program::Program,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Addr(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CpuState {
    Running,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct Memory {
    mem: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RegSet {
    regs: HashMap<ArchReg, u32>,
}

#[derive(Debug, Clone)]
pub struct Cpu {
    pub regs: RegSet,
    pub mem: Memory,
    pub prog: Program,
    pub ip: u32,
    pub cycles: u64,
}

impl Cpu {
    pub fn new(prog: Program, in_regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        assert!(in_regs.get(&ArchReg::Zero).is_none());

        Cpu {
            regs: RegSet { regs: in_regs },
            ip: 0,
            cycles: 0,
            mem,
            prog,
        }
    }

    pub fn exec_one(&mut self) -> CpuState {
        let next_inst = match self.prog.insts.get(usize::try_from(self.ip).unwrap()) {
            Some(i) => i,
            None => return CpuState::Stopped,
        };

        let mut advance_ip = true;

        match *next_inst {
            Inst::LoadByte(dst, src) => {
                let val = self.mem.readb(self.regs.ref_to_addr(src));
                self.regs.set(dst, val);
            }
            Inst::LoadHalfWord(dst, src) => {
                let val = self.mem.readh(self.regs.ref_to_addr(src));
                self.regs.set(dst, val);
            }
            Inst::LoadWord(dst, src) => {
                let val = self.mem.readw(self.regs.ref_to_addr(src));
                self.regs.set(dst, val);
            }
            Inst::StoreByte(dst, src) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeb(dst, self.regs.get(src));
            }
            Inst::StoreHalfWord(dst, src) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeh(dst, self.regs.get(src));
            }
            Inst::StoreWord(dst, src) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writew(dst, self.regs.get(src));
            }
            Inst::Add(dst, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                self.regs.set(dst, a.wrapping_add(b));
            }
            Inst::AddImm(dst, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                self.regs.set(dst, a.wrapping_add(b));
            }
            Inst::JumpAndLink(dst, ref offset) => {
                self.regs.set(dst, self.ip + 1);
                self.ip += offset.0;
                advance_ip = false;
            }
            Inst::BranchIfEqual(src0, src1, ref dst) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a == b {
                    self.ip = self.prog.labels[dst];
                    advance_ip = false;
                }
            }
            Inst::BranchIfNotEqual(src0, src1, ref dst) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a != b {
                    self.ip = self.prog.labels[dst];
                    advance_ip = false;
                }
            }
        }

        if advance_ip {
            self.ip += 1;
        }

        self.cycles += 1;

        CpuState::Running
    }

    pub fn exec_all(mut self) -> Cpu {
        while CpuState::Running == self.exec_one() {
            // dbg!(&self.regs);
        }
        self
    }
}

impl RegSet {
    pub fn get(&self, reg: ArchReg) -> u32 {
        if reg == ArchReg::Zero {
            0
        } else {
            *self.regs.get(&reg).unwrap_or(&0)
        }
    }

    pub fn set(&mut self, reg: ArchReg, value: u32) {
        if reg != ArchReg::Zero {
            self.regs.insert(reg, value);
        }
    }

    pub fn ref_to_addr(&self, mr: MemRef) -> Addr {
        Addr(self.get(mr.base).wrapping_add(mr.offset.0))
    }
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
