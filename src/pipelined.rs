use crate::{
    cpu::{Cpu, ExecResult},
    inst::{ArchReg, Inst},
    mem::Memory,
    program::Program,
    regs::RegSet,
    util::Addr,
};
use std::{collections::HashMap, default::Default};

mod stages {
    use super::*;

    #[derive(Debug, Default, Clone)]
    pub struct Fetch {
        pub inst: Option<Inst>,
        pub next_pc: u32,
    }

    #[derive(Debug, Default)]
    pub struct Decode {
        pub inst: Option<Inst>,
        pub should_stall: bool,
    }

    #[derive(Debug, Default)]
    pub struct Execute {
        pub inst: Option<Inst>,
        pub should_stall: bool,
        // pub cycles_spent: u64,
        pub alu_or_mem_val: u32,
        pub mem_addr: Addr,
    }

    #[derive(Debug, Default)]
    pub struct Memory {
        pub inst: Option<Inst>,
        pub alu_or_mem_val: u32,
    }

    #[derive(Debug, Default)]
    pub struct Writeback {
        pub jump_target: Option<u32>,
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Default)]
struct Pipeline {
    fetch: stages::Fetch,
    decode: stages::Decode,
    execute: stages::Execute,
    memory: stages::Memory,
    writeback: stages::Writeback,
}

#[derive(Debug, Clone)]
pub struct Pipelined {
    regs: RegSet,
    mem: Memory,
    prog: Program,
    is_stalled: bool,
}

impl Cpu for Pipelined {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        assert!(regs.get(&ArchReg::Zero).is_none());

        Self {
            regs: RegSet::new(regs),
            is_stalled: false,
            mem,
            prog,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut cycles = 0;
        let mut insts_retired = 0;
        let mut pipe = Pipeline::default();

        loop {
            let fetch = self.stage_fetch(&pipe);
            let decode = self.stage_decode(&pipe);
            let execute = self.stage_execute(&pipe);
            let memory = self.stage_memory(&pipe);
            let writeback = self.stage_writeback(&pipe);

            if writeback.should_halt {
                return ExecResult {
                    mem: self.mem,
                    cycles_taken: cycles,
                    insts_retired,
                };
            }

            if writeback.retire {
                insts_retired += 1;
            }

            if execute.should_stall {
                pipe = Pipeline {
                    fetch: pipe.fetch,
                    decode: pipe.decode,
                    execute,
                    memory,
                    writeback,
                };
            } else if decode.should_stall {
                self.is_stalled = true;
                pipe = Pipeline {
                    fetch: stages::Fetch {
                        inst: None,
                        next_pc: pipe.fetch.next_pc,
                    },
                    decode,
                    execute,
                    memory,
                    writeback,
                };
            } else {
                pipe = Pipeline {
                    fetch,
                    decode,
                    execute,
                    memory,
                    writeback,
                };
            }
            dbg!(&pipe);

            cycles += 1;

            if std::env::var("SINGLE_STEP").is_ok() {
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }

            // debug_assert!(cycles < 10, "infinite loop detected");
            debug_assert!(cycles < 10_000, "infinite loop detected");
        }
    }
}

impl Pipelined {
    fn stage_fetch(&mut self, pipe: &Pipeline) -> stages::Fetch {
        let fetch_or_halt = |pc| self.prog.fetch(pc).cloned().unwrap_or(Inst::Halt);

        if let Some(tgt) = pipe.writeback.jump_target {
            debug_assert!(self.is_stalled);
            self.is_stalled = false;
            stages::Fetch {
                inst: Some(fetch_or_halt(tgt)),
                next_pc: tgt + 1,
            }
        } else if self.is_stalled {
            // Continue being stalled.
            pipe.fetch.clone()
        } else {
            stages::Fetch {
                inst: Some(fetch_or_halt(pipe.fetch.next_pc)),
                next_pc: pipe.fetch.next_pc + 1,
            }
        }
    }

    fn stage_decode(&self, pipe: &Pipeline) -> stages::Decode {
        let inst = match &pipe.fetch.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        if inst.is_branch() {
            stages::Decode {
                should_stall: true,
                inst: Some(inst),
            }
        } else {
            stages::Decode {
                should_stall: false,
                inst: Some(inst),
            }
        }
    }

    fn stage_execute(&self, pipe: &Pipeline) -> stages::Execute {
        if hazard::read_after_write(&pipe.decode.inst, &pipe.execute.inst)
            || hazard::read_after_write(&pipe.decode.inst, &pipe.memory.inst)
        {
            return stages::Execute {
                should_stall: true,
                ..Default::default()
            };
        }

        let inst = match &pipe.decode.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        let mut out = stages::Execute::default();

        match inst {
            Inst::Add(_, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu_or_mem_val = a.wrapping_add(b);
            }
            Inst::AddImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu_or_mem_val = a.wrapping_add(b);
            }
            Inst::ShiftLeftLogicalImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu_or_mem_val = a.wrapping_shl(b);
            }
            Inst::BranchIfEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu_or_mem_val = (a == b).into();
            }
            Inst::BranchIfNotEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu_or_mem_val = (a != b).into();
            }
            Inst::BranchIfGreaterEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu_or_mem_val = (a >= b).into();
            }
            Inst::LoadByte(_, addr) | Inst::LoadHalfWord(_, addr) | Inst::LoadWord(_, addr) => {
                out.mem_addr = self.regs.ref_to_addr(addr);
            }
            Inst::StoreByte(src, dst)
            | Inst::StoreHalfWord(src, dst)
            | Inst::StoreWord(src, dst) => {
                out.mem_addr = self.regs.ref_to_addr(dst);
                out.alu_or_mem_val = self.regs.get(src);
            }
            Inst::Halt => (),
            _ => unimplemented!("{:?}", inst),
        }

        out.inst = Some(inst);
        out
    }

    fn stage_memory(&mut self, pipe: &Pipeline) -> stages::Memory {
        let inst = match &pipe.execute.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        let addr = pipe.execute.mem_addr;
        let mut val = pipe.execute.alu_or_mem_val;

        match inst {
            Inst::LoadByte(_, _) => {
                val = self.mem.readb(addr);
            }
            Inst::LoadHalfWord(_, _) => {
                val = self.mem.readh(addr);
            }
            Inst::LoadWord(_, _) => {
                val = self.mem.readw(addr);
            }
            Inst::StoreByte(_, _) => {
                self.mem.writeb(addr, val);
            }
            Inst::StoreHalfWord(_, _) => {
                self.mem.writeh(addr, val);
            }
            Inst::StoreWord(_, _) => {
                self.mem.writew(addr, val);
            }
            ref x => debug_assert!(!x.is_mem_access()),
        }

        stages::Memory {
            inst: Some(inst),
            alu_or_mem_val: val,
        }
    }

    fn stage_writeback(&mut self, pipe: &Pipeline) -> stages::Writeback {
        let inst = match &pipe.memory.inst {
            Some(Inst::Halt) => {
                return stages::Writeback {
                    jump_target: None,
                    should_halt: true,
                    retire: false,
                }
            }
            Some(inst) => inst.clone(),
            None => {
                return stages::Writeback {
                    jump_target: None,
                    should_halt: false,
                    retire: false,
                }
            }
        };

        let val = pipe.memory.alu_or_mem_val;
        let mut jump_target = None;

        match inst {
            Inst::ShiftLeftLogicalImm(dst, _, _)
            | Inst::LoadByte(dst, _)
            | Inst::LoadHalfWord(dst, _)
            | Inst::LoadWord(dst, _)
            | Inst::Add(dst, _, _)
            | Inst::AddImm(dst, _, _) => {
                self.regs.set(dst, val);
            }
            Inst::BranchIfNotEqual(_, _, ref dst)
            | Inst::BranchIfEqual(_, _, ref dst)
            | Inst::BranchIfGreaterEqual(_, _, ref dst) => {
                jump_target = if val != 0 {
                    Some(self.prog.labels[dst])
                } else {
                    Some(pipe.fetch.next_pc)
                };
            }
            // The other memory accesses are handled in the previous pipeline stage
            x if x.is_mem_access() => (),
            _ => unimplemented!("{:?}", inst),
        }

        stages::Writeback {
            jump_target,
            should_halt: false,
            retire: true,
        }
    }
}

mod hazard {
    use super::*;

    pub fn read_after_write(a: &Option<Inst>, b: &Option<Inst>) -> bool {
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) => (a, b),
            _ => return false,
        };

        match *a {
            Inst::StoreByte(src, dst)
            | Inst::StoreHalfWord(src, dst)
            | Inst::StoreWord(src, dst) => b.writes_reg(src) || b.writes_reg(dst.base),
            Inst::LoadByte(_, src) | Inst::LoadHalfWord(_, src) | Inst::LoadWord(_, src) => {
                b.writes_reg(src.base)
            }
            Inst::BranchIfNotEqual(src0, src1, _)
            | Inst::BranchIfEqual(src0, src1, _)
            | Inst::BranchIfGreaterEqual(src0, src1, _)
            | Inst::Add(_, src0, src1) => b.writes_reg(src0) || b.writes_reg(src1),
            Inst::ShiftLeftLogicalImm(_, src, _) | Inst::AddImm(_, src, _) => b.writes_reg(src),
            Inst::JumpAndLink(_, _) | Inst::Halt => false,
        }
    }
}
