use std::collections::{HashMap};

use crate::{
    cpu::{Cpu, ExecResult},
    execution_unit::{EuType, ExecutionUnit},
    inst::{ArchReg, BothReg, Inst, PhysReg, RenamedInst, Tag, ValueOrReg},
    mem::Memory,
    program::Program,
    regs::RegFile,
    rob::ReorderBuffer,
};

mod stages {
    use crate::inst::{ExecutedInst};

    use super::*;

    #[derive(Debug, Clone, Default)]
    pub struct Fetch {
        pub inst: Option<Inst>,
        pub next_pc: u32,
    }

    #[derive(Debug, Clone, Default)]
    pub struct DecodeIssue {
        pub inst: Option<Inst>,
        pub should_stall: bool,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Execute;

    #[derive(Debug, Clone, Default)]
    pub struct Writeback {
        pub inst: Option<ExecutedInst>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Commit {
        pub inst: Option<Inst>,
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Clone, Default)]
#[allow(unused)]
pub struct Pipeline {
    fetch: stages::Fetch,
    decode_issue: stages::DecodeIssue,
    execute: stages::Execute,
    writeback: stages::Writeback,
    commit: stages::Commit,
}

#[derive(Debug, Clone)]
pub struct OutOfOrder {
    mem: Memory,
    prog: Program,
    execution_units: Vec<ExecutionUnit>,
    reservation_station: HashMap<Tag, RenamedInst>,
    rob: ReorderBuffer,
    reg_file: RegFile,
    cycles: u64,
    rs_max: usize,
}

impl Cpu for OutOfOrder {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        Self {
            mem,
            prog,
            execution_units: vec![
                ExecutionUnit::new(EuType::ALU),
                ExecutionUnit::new(EuType::LoadStore),
                ExecutionUnit::new(EuType::Special),
            ],
            rob: ReorderBuffer::new(20),
            reservation_station: Default::default(),
            reg_file: RegFile::new(regs, 32),
            rs_max: 10,
            cycles: 0,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut insts_retired = 0;
        let mut pipe = Pipeline::default();

        loop {
            let fetch = self.stage_fetch(&pipe);
            let decode_issue = self.stage_decode_issue(&pipe);
            let execute = self.stage_execute(&pipe);
            let writeback = self.stage_writeback(&pipe);
            let commit = self.stage_commit(&pipe);

            if commit.should_halt {
                return ExecResult {
                    mem: self.mem,
                    cycles_taken: self.cycles,
                    insts_retired,
                };
            }

            if commit.retire {
                insts_retired += 1;
            }

            if decode_issue.should_stall {
                pipe = Pipeline {
                    fetch: pipe.fetch,
                    decode_issue,
                    execute,
                    writeback,
                    commit,
                };
            } else {
                pipe = Pipeline {
                    fetch,
                    decode_issue,
                    execute,
                    writeback,
                    commit,
                };
            }

            self.cycles += 1;

            if std::env::var("SINGLE_STEP").is_ok() {
                self.dump(&pipe);
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }

            // debug_assert!(self.cycles < 10, "infinite loop detected");
            debug_assert!(self.cycles < 10_000, "infinite loop detected");
        }
    }
}

impl OutOfOrder {
    #[allow(dead_code)]
    fn dump(&self, pipe: &Pipeline) {
        dbg!(&self.reg_file);
        dbg!(&self.rob);
        dbg!(&self.reservation_station);
        dbg!(&self.execution_units);
        dbg!(pipe);
    }

    fn rename_and_reserve(&mut self, inst: Inst) -> bool {
        // Assumes we only issue 1 inst per cycle.
        let tag = Tag::from(self.cycles);

        if self.rob.is_full() || self.reservation_station.len() == self.rs_max {
            return true;
        }

        let mut should_stall = false;

        let renamed_inst = inst.clone().map_src_regs(|src_reg| match src_reg {
            ArchReg::Zero => ValueOrReg::Value(0),
            src_reg => ValueOrReg::Reg(self.reg_file.get_alias(src_reg)),
        });
        let renamed_inst = renamed_inst.map_dst_regs(|dst_reg| {
            if dst_reg == ArchReg::Zero {
                BothReg {
                    arch: ArchReg::Zero,
                    phys: 0, // Doesn't matter, (?)
                }
            } else if let Some(slot) = self.reg_file.allocate_phys() {
                self.reg_file.set_alias(dst_reg, slot);
                BothReg {
                    arch: dst_reg,
                    phys: slot,
                }
            } else {
                should_stall = true; // PRF full, need to stall.
                BothReg {
                    arch: dst_reg,
                    phys: PhysReg::default(),
                }
            }
        });

        if should_stall {
            return true;
        }

        assert_eq!(self.rob.try_push(tag, inst), None);
        assert_eq!(self.reservation_station.insert(tag, renamed_inst), None);
        false
    }

    fn issue(&mut self, inst: Inst) -> bool {
        // Try to issue more instructions to reservation stations.
        let mut remove_tags = vec![];

        for (tag, ready_inst) in self
            .reservation_station
            .iter()
            .filter_map(|(&tag, inst)| inst.get_ready(&self.reg_file).map(|inst| (tag, inst)))
        {
            if let Some(eu) = self
                .execution_units
                .iter_mut()
                .find(|eu| eu.can_execute(&ready_inst))
            {
                // Enforce that stores and loads must execute in program order.
                if ready_inst.is_mem_access() && !self.rob.is_earliest_mem_access(tag) {
                    continue;
                }

                remove_tags.push(tag);
                eu.begin_execute(ready_inst, tag);
            }
        }

        for tag in remove_tags {
            self.reservation_station.remove(&tag);
        }

        if self.rename_and_reserve(inst) {
            return true;
        }

        false
    }

    fn stage_fetch(&mut self, pipe: &Pipeline) -> stages::Fetch {
        let fetch_or_halt = |pc| self.prog.fetch(pc).cloned().unwrap_or(Inst::Halt);

        stages::Fetch {
            inst: Some(fetch_or_halt(pipe.fetch.next_pc)),
            next_pc: pipe.fetch.next_pc + 1, // TODO: branch prediction
        }
    }

    fn stage_decode_issue(&mut self, pipe: &Pipeline) -> stages::DecodeIssue {
        let inst = match &pipe.fetch.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        // Give instruction to the scheduler to rename and be placed into an execution unit.
        let should_stall = self.issue(inst.clone());

        stages::DecodeIssue {
            inst: Some(inst),
            should_stall,
        }
    }

    fn stage_execute(&mut self, _pipe: &Pipeline) -> stages::Execute {
        // Advance execution of all the execution units.
        for eu in &mut self.execution_units {
            eu.advance(&mut self.mem);
        }

        stages::Execute
    }

    fn stage_writeback(&mut self, _pipe: &Pipeline) -> stages::Writeback {
        // Take output of the execution units and write into the ROB.

        for eu in &mut self.execution_units {
            if let Some((inst, tag, result)) = eu.take_complete() {
                // Only allow writeback of 1 instruction per cycle, for now.
                let mut completed_reg = 0;

                match &inst {
                    Inst::AddImm(dst, _, _) | Inst::LoadWord(dst, _) => {
                        completed_reg = dst.phys;
                        self.reg_file.set_phys_active(dst.phys, result.val);
                    }
                    Inst::StoreWord(_, _) | Inst::Halt => (),
                    _ => unimplemented!("{:?}", inst),
                }

                // Broadcast tag of completed instruction to waiting instructions.
                for (_, waiting_inst) in self.reservation_station.iter_mut() {
                    *waiting_inst = waiting_inst.clone().map_src_regs(|src_reg| {
                        if src_reg == ValueOrReg::Reg(completed_reg) {
                            ValueOrReg::Value(result.val)
                        } else {
                            src_reg
                        }
                    });
                }

                self.rob.mark_complete(tag);

                return stages::Writeback { inst: Some(inst) };
            }
        }

        stages::Writeback { inst: None }
    }

    fn stage_commit(&mut self, _pipe: &Pipeline) -> stages::Commit {
        // Commit instructions from the ROB to architectural state.
        let inst = match self.rob.try_pop() {
            i @ Some(Inst::Halt) => {
                return stages::Commit {
                    inst: i,
                    should_halt: true,
                    retire: false,
                }
            }
            Some(inst) => inst,
            None => return Default::default(),
        };

        match inst {
            Inst::AddImm(dst, _, _) | Inst::LoadWord(dst, _) => {
                if dst != ArchReg::Zero {
                    self.reg_file.release_phys();
                }
            }
            Inst::StoreWord(_, _) => (),
            _ => unimplemented!("{:?}", inst),
        }

        stages::Commit {
            inst: Some(inst),
            should_halt: false,
            retire: true,
        }
    }
}
