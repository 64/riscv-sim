use std::collections::{BTreeMap, HashMap};

use crate::{
    branch::{BranchPredictor, BranchTargetBuffer},
    cpu::{Cpu, ExecResult},
    execution_unit::{EuType, ExecutionUnit},
    inst::{ArchReg, Inst, RenamedInst, Tag, Tagged},
    lsq::LoadStoreQueue,
    mem::Memory,
    program::Program,
    regs::RegFile,
    rob::ReorderBuffer,
};

mod stages {
    use crate::inst::{ExecutedInst, Tagged};

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
        pub inst: Option<Tagged<ExecutedInst>>,
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
    reservation_station: BTreeMap<Tag, RenamedInst>,
    lsq: LoadStoreQueue,
    rob: ReorderBuffer,
    btb: BranchTargetBuffer,
    branch_predictor: BranchPredictor,
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
                ExecutionUnit::new(EuType::ALU),
                ExecutionUnit::new(EuType::LoadStore),
                ExecutionUnit::new(EuType::Special),
            ],
            rob: ReorderBuffer::new(20),
            lsq: LoadStoreQueue::new(10, 10),
            reservation_station: Default::default(),
            reg_file: RegFile::new(regs, 32),
            btb: BranchTargetBuffer::new(50),
            branch_predictor: BranchPredictor::new(),
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

        if inst.is_mem_access() && !self.lsq.has_space(&inst) {
            return true;
        }

        if let Some(renamed_inst) = self.reg_file.perform_rename(inst.clone()) {
            assert_eq!(self.rob.try_push(tag, inst), None);
            assert_eq!(
                self.reservation_station.insert(tag, renamed_inst.clone()),
                None
            );

            if renamed_inst.is_mem_access() {
                self.lsq.insert_access(renamed_inst, tag);
            }

            false
        } else {
            true // Renaming failed because we need to stall.
        }
    }

    fn stage_fetch(&mut self, pipe: &Pipeline) -> stages::Fetch {
        let fetch_or_halt = |pc| self.prog.fetch(pc).cloned().unwrap_or(Inst::Halt);

        // Branch prediction
        let pc = pipe.fetch.next_pc;
        let next_pc = match self.btb.get(pc) {
            Some(target) if self.branch_predictor.predict_taken(pc, target) => target,
            _ => pc + 1,
        };

        stages::Fetch {
            inst: Some(fetch_or_halt(pc)),
            next_pc,
        }
    }

    fn stage_decode_issue(&mut self, pipe: &Pipeline) -> stages::DecodeIssue {
        match &pipe.fetch.inst {
            Some(inst) => stages::DecodeIssue {
                inst: Some(inst.clone()),
                should_stall: self.issue(inst.clone()),
            },
            None => Default::default(),
        }
    }

    // Advance execution of all the execution units.
    fn stage_execute(&mut self, _pipe: &Pipeline) -> stages::Execute {
        for eu in &mut self.execution_units {
            eu.advance(&mut self.mem);
        }

        stages::Execute
    }

    // Take output of the execution units and write into the register file.
    fn stage_writeback(&mut self, _pipe: &Pipeline) -> stages::Writeback {
        for eu in &mut self.execution_units {
            if let Some((Tagged { tag, inst }, result)) = eu.take_complete() {
                // Only allow writeback of 1 instruction per cycle, for now.
                match &inst {
                    Inst::AddImm(dst, _, _) | Inst::LoadWord(dst, _) => {
                        if dst.arch != ArchReg::Zero {
                            self.reg_file.set_phys_active(dst.phys, result.val);
                        }
                    }
                    Inst::BranchIfEqual(_, _, tgt)
                    | Inst::BranchIfNotEqual(_, _, tgt)
                    | Inst::BranchIfGreaterEqual(_, _, tgt) => {
                        todo!(); // Check if our prediction was correct.
                    }
                    Inst::StoreWord(_, _) | Inst::Halt => (),
                    _ => unimplemented!("{:?}", inst),
                }

                return stages::Writeback {
                    inst: Some(Tagged { tag, inst }),
                };
            }
        }

        stages::Writeback { inst: None }
    }

    // Commit instructions from the ROB to architectural state.
    fn stage_commit(&mut self, pipe: &Pipeline) -> stages::Commit {
        let tagged = self.rob.try_pop();

        // Mark the written back instructions as completed now, to simulate 1 cycle of delay
        // between writeback and commit.
        if let Some(tagged) = &pipe.writeback.inst {
            self.rob.mark_complete(tagged.tag);
        }

        let Tagged { tag, inst } = match tagged {
            Some(Tagged {
                inst: Inst::Halt, ..
            }) => {
                return stages::Commit {
                    inst: Some(Inst::Halt),
                    should_halt: true,
                    retire: false,
                }
            }
            Some(tagged) => tagged,
            None => return Default::default(),
        };

        match inst {
            Inst::AddImm(dst, _, _) | Inst::LoadWord(dst, _) => {
                if dst != ArchReg::Zero {
                    self.reg_file.release_phys();
                }
            }
            Inst::StoreWord(_, _) => self.lsq.submit_store(tag, &self.reg_file, &mut self.mem),
            _ => unimplemented!("{:?}", inst),
        }

        stages::Commit {
            inst: Some(inst),
            should_halt: false,
            retire: true,
        }
    }

    fn issue(&mut self, inst: Inst) -> bool {
        // Try to issue more instructions to reservation stations.
        let mut remove_tags = vec![];

        if self.rename_and_reserve(inst) {
            return true;
        }

        // TODO: improve this logic.
        for (tag, ready_inst) in self
            .reservation_station
            .iter()
            .filter_map(|(&tag, inst)| inst.get_ready(&self.reg_file).map(|inst| (tag, inst)))
        {
            if ready_inst.is_load() && !self.lsq.can_execute_load(tag) {
                continue;
            }

            if let Some(eu) = self
                .execution_units
                .iter_mut()
                .find(|eu| eu.can_execute(&ready_inst))
            {
                remove_tags.push(tag);
                eu.begin_execute(ready_inst, tag);
            }
        }

        for tag in remove_tags {
            self.reservation_station.remove(&tag);
        }

        false
    }

    /// Get a reference to the out of order's branch predictor.
    pub fn branch_predictor(&self) -> &BranchPredictor {
        &self.branch_predictor
    }

    /// Get a reference to the out of order's btb.
    pub fn btb(&self) -> &BranchTargetBuffer {
        &self.btb
    }
}
