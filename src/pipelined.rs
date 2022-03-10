use crate::{
    cpu::{Cpu, ExecResult},
    inst::{ArchReg, Inst},
    mem::Memory,
    program::Program,
    regs::RegSet,
};
use std::collections::HashMap;

mod stages {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Fetch {
        pub inst: Option<Inst>,
        pub new_pc: u32,
    }

    #[derive(Debug, Default)]
    pub struct Decode {
        pub inst: Option<Inst>,
        pub should_stall: bool,
    }

    #[derive(Debug, Default)]
    pub struct ExMem {
        pub inst: Option<Inst>,
        pub alu: u32,
        pub mem: u32,
    }

    #[derive(Debug, Default)]
    pub struct MemWb {
        pub jump_target: Option<u32>,
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Default)]
struct Pipeline {
    fetch: stages::Fetch,
    decode: stages::Decode,
    ex_mem: stages::ExMem,
    mem_wb: stages::MemWb,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcMode {
    Inc,
    Stalled,
}

#[derive(Debug, Clone)]
pub struct Pipelined {
    regs: RegSet,
    mem: Memory,
    prog: Program,
    pc_mode: PcMode,
}

impl Cpu for Pipelined {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        assert!(regs.get(&ArchReg::Zero).is_none());

        Self {
            regs: RegSet::new(regs),
            pc_mode: PcMode::Inc,
            mem,
            prog,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut cycles = 0;
        let mut insts_retired = 0;
        let mut pipe = Pipeline::default();

        loop {
            let mut fetch = self.stage_fetch(&pipe);
            let decode = self.stage_decode(&pipe);
            let ex_mem = self.stage_ex_mem(&pipe);
            let mem_wb = self.stage_mem_wb(&pipe);

            if decode.should_stall {
                self.pc_mode = PcMode::Stalled;

                // Flush. Need to decrement new_pc since it was incremented after
                // fetching the branch which caused the stall.
                fetch = stages::Fetch {
                    inst: None,
                    new_pc: pipe.fetch.new_pc - 1,
                };
            }

            if mem_wb.should_halt {
                return ExecResult {
                    mem: self.mem,
                    cycles_taken: cycles,
                    insts_retired,
                };
            }

            if mem_wb.retire {
                insts_retired += 1;
            }

            pipe = Pipeline {
                fetch,
                decode,
                ex_mem,
                mem_wb,
            };
            dbg!(&pipe);

            cycles += 1;
            // debug_assert!(cycles < 10, "infinite loop detected");
            debug_assert!(cycles < 1_000, "infinite loop detected");
        }
    }
}

impl Pipelined {
    fn stage_fetch(&mut self, pipe: &Pipeline) -> stages::Fetch {
        let cur_pc = pipe.fetch.new_pc;
        let fetch_or_halt = |pc| self.prog.fetch(pc).cloned().unwrap_or(Inst::Halt);

        match (pipe.mem_wb.jump_target, &self.pc_mode) {
            (Some(tgt), _) => {
                self.pc_mode = PcMode::Inc;
                stages::Fetch {
                    inst: Some(fetch_or_halt(tgt)),
                    new_pc: tgt + 1,
                }
            }
            (None, PcMode::Stalled) => stages::Fetch {
                inst: None,
                new_pc: cur_pc,
            },
            (None, PcMode::Inc) => stages::Fetch {
                inst: Some(fetch_or_halt(cur_pc)),
                new_pc: cur_pc + 1,
            }
        }
    }

    fn stage_decode(&mut self, pipe: &Pipeline) -> stages::Decode {
        let inst = match &pipe.fetch.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        stages::Decode {
            should_stall: inst.is_branch(),
            inst: Some(inst),
        }
    }

    fn stage_ex_mem(&mut self, pipe: &Pipeline) -> stages::ExMem {
        let inst = match &pipe.decode.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        let mut out = stages::ExMem::default();

        match inst {
            Inst::StoreByte(src, _) | Inst::StoreHalfWord(src, _) | Inst::StoreWord(src, _) => {
                out.mem = self.regs.get(src)
            }
            Inst::LoadByte(_, src) => {
                out.mem = self.mem.readb(self.regs.ref_to_addr(src));
            }
            Inst::LoadHalfWord(_, src) => {
                out.mem = self.mem.readh(self.regs.ref_to_addr(src));
            }
            Inst::LoadWord(_, src) => {
                out.mem = self.mem.readw(self.regs.ref_to_addr(src));
            }
            Inst::Add(_, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = a.wrapping_add(b);
            }
            Inst::AddImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu = a.wrapping_add(b);
            }
            Inst::ShiftLeftLogicalImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu = a.wrapping_shl(b);
            }
            Inst::BranchIfEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = (a == b).into();
            }
            Inst::BranchIfNotEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = (a != b).into();
            }
            Inst::BranchIfGreaterEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = (a >= b).into();
            }
            Inst::Halt => (),
            _ => unimplemented!("{:?}", inst),
        }

        out.inst = Some(inst);
        out
    }

    fn stage_mem_wb(&mut self, pipe: &Pipeline) -> stages::MemWb {
        let inst = match &pipe.ex_mem.inst {
            Some(Inst::Halt) => {
                return stages::MemWb {
                    jump_target: None,
                    should_halt: true,
                    retire: false,
                }
            }
            Some(inst) => inst.clone(),
            None => {
                return stages::MemWb {
                    jump_target: None,
                    should_halt: false,
                    retire: false,
                }
            }
        };

        let mut jump_target = None;

        match inst {
            Inst::StoreByte(_, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeb(dst, pipe.ex_mem.mem);
            }
            Inst::StoreHalfWord(_, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeh(dst, pipe.ex_mem.mem);
            }
            Inst::StoreWord(_, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writew(dst, pipe.ex_mem.mem);
            }
            Inst::LoadByte(dst, _) | Inst::LoadHalfWord(dst, _) | Inst::LoadWord(dst, _) => {
                self.regs.set(dst, pipe.ex_mem.mem);
            }
            Inst::ShiftLeftLogicalImm(dst, _, _)
            | Inst::Add(dst, _, _)
            | Inst::AddImm(dst, _, _) => {
                self.regs.set(dst, pipe.ex_mem.alu);
            }
            Inst::BranchIfNotEqual(_, _, ref dst)
            | Inst::BranchIfEqual(_, _, ref dst)
            | Inst::BranchIfGreaterEqual(_, _, ref dst) => {
                jump_target = if pipe.ex_mem.alu != 0 {
                    Some(self.prog.labels[dst])
                } else {
                    Some(pipe.fetch.new_pc + 1)
                };
            }
            _ => unimplemented!("{:?}", inst),
        }

        stages::MemWb {
            jump_target,
            should_halt: false,
            retire: true,
        }
    }
}
