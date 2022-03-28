use crate::{
    cpu::{Cpu, CpuState, ExecResult},
    inst::Inst,
    mem::MainMemory,
    program::Program,
    regs::RegSet,
};

#[derive(Debug, Clone)]
pub struct Emulated {
    regs: RegSet,
    mem: MainMemory,
    prog: Program,
    pc: u32,
    cycles: u64,
    insts_retired: u64,
}

impl Cpu for Emulated {
    fn new(prog: Program, regs: RegSet, mem: MainMemory) -> Self {
        Self {
            pc: 0,
            cycles: 0,
            insts_retired: 0,
            regs,
            mem,
            prog,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        while CpuState::Running == self.exec_one() {
            #[cfg(debug_assertions)]
            if std::env::var("VERBOSE").is_ok() {
                dbg!(&self.regs);
            }
        }

        ExecResult {
            mem: self.mem,
            regs: self.regs,
            cycles_taken: self.cycles,
            insts_retired: self.insts_retired,
        }
    }
}

impl Emulated {
    fn exec_one(&mut self) -> CpuState {
        let next_inst = match self.prog.fetch(self.pc) {
            Some(i) => i,
            None => return CpuState::Stopped,
        };

        if std::env::var("SINGLE_STEP").is_ok() {
            println!("{:?}", next_inst);
            std::io::stdin().read_line(&mut String::new()).unwrap();
        }

        let mut advance_pc = true;

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
            Inst::StoreByte(src, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeb(dst, self.regs.get(src));
            }
            Inst::StoreHalfWord(src, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeh(dst, self.regs.get(src));
            }
            Inst::StoreWord(src, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writew(dst, self.regs.get(src));
            }
            Inst::Add(dst, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                self.regs.set(dst, a.wrapping_add(b));
            }
            Inst::Sub(dst, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                self.regs.set(dst, a.wrapping_sub(b));
            }
            Inst::AddImm(dst, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                self.regs.set(dst, a.wrapping_add(b));
            }
            Inst::AndImm(dst, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                self.regs.set(dst, a & b);
            }
            Inst::ShiftLeftLogicalImm(dst, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                self.regs.set(dst, a.wrapping_shl(b));
            }
            Inst::Mul(dst, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                self.regs.set(dst, a.wrapping_mul(b));
            }
            Inst::Rem(dst, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                let val = if b == 0 { a } else { a % b };
                self.regs.set(dst, val);
            }
            Inst::Jump(tgt) => {
                self.pc = tgt.into();
                advance_pc = false;
            }
            Inst::JumpAndLink(dst, tgt) => {
                self.regs.set(dst, self.pc + 1);
                self.pc = tgt.into();
                advance_pc = false;
            }
            Inst::BranchIfEqual(src0, src1, tgt) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a == b {
                    self.pc = tgt.into();
                    advance_pc = false;
                }
            }
            Inst::BranchIfNotEqual(src0, src1, tgt) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a != b {
                    self.pc = tgt.into();
                    advance_pc = false;
                }
            }
            Inst::BranchIfGreaterEqual(src0, src1, tgt) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a >= b {
                    self.pc = tgt.into();
                    advance_pc = false;
                }
            }
            Inst::BranchIfLess(src0, src1, tgt) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                if a < b {
                    self.pc = tgt.into();
                    advance_pc = false;
                }
            }
            Inst::SetLessThanImmU(dst, src, imm) => {
                let r = self.regs.get(src);
                let val = if r < imm.0 { 1 } else { 0 };
                self.regs.set(dst, val);
            }
            Inst::Halt => unreachable!(),
        }

        if advance_pc {
            self.pc += 1;
        }

        self.insts_retired += 1;
        self.cycles += next_inst.latency();

        CpuState::Running
    }
}
