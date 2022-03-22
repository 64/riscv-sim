use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    branch::BranchPredictor,
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
    pub struct FetchDecode {
        pub inst: Option<Tagged<Inst>>,
        pub next_pc: u32,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Rename {
        pub inst: Option<RenamedInst>,
        pub should_stall: bool,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Issue;

    #[derive(Debug, Clone, Default)]
    pub struct Execute;

    #[derive(Debug, Clone, Default)]
    pub struct Writeback {
        pub inst: Option<Tagged<ExecutedInst>>,
        pub next_fetch: Option<u32>,
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
    fetch_decode: stages::FetchDecode,
    rename: stages::Rename,
    issue: stages::Issue,
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
                ExecutionUnit::new(EuType::Branch),
                ExecutionUnit::new(EuType::LoadStore),
                ExecutionUnit::new(EuType::Special),
            ],
            rob: ReorderBuffer::new(20),
            lsq: LoadStoreQueue::new(10, 10),
            reservation_station: Default::default(),
            reg_file: RegFile::new(regs, 18),
            branch_predictor: BranchPredictor::new(),
            rs_max: 10,
            cycles: 0,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut insts_retired = 0;
        let mut pipe = Pipeline::default();

        loop {
            let commit = self.stage_commit(&pipe);
            let writeback = self.stage_writeback(&pipe);
            let execute = self.stage_execute(&pipe);
            let issue = self.stage_issue(&pipe);

            if commit.should_halt {
                return ExecResult {
                    regs: self.reg_file.get_reg_set(),
                    mem: self.mem,
                    cycles_taken: self.cycles,
                    insts_retired,
                };
            }

            if commit.retire {
                insts_retired += 1;
            }

            if let Some(next_pc) = writeback.next_fetch {
                pipe = Pipeline {
                    fetch_decode: stages::FetchDecode {
                        inst: None,
                        next_pc,
                    },
                    rename: stages::Rename::default(),
                    issue,
                    execute,
                    writeback,
                    commit,
                };
            } else {
                let rename = self.stage_rename(&pipe);
                let fetch_decode = self.stage_fetch_decode(&pipe);

                if rename.should_stall {
                    pipe = Pipeline {
                        fetch_decode: pipe.fetch_decode,
                        rename,
                        issue,
                        execute,
                        writeback,
                        commit,
                    };
                } else {
                    pipe = Pipeline {
                        fetch_decode,
                        rename,
                        issue,
                        execute,
                        writeback,
                        commit,
                    };
                }
            }

            self.cycles += 1;

            if std::env::var("SINGLE_STEP").is_ok() {
                self.dump(&pipe);
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }

            debug_assert!(self.cycles < 100_000, "infinite loop detected");
        }
    }
}

impl OutOfOrder {
    #[allow(dead_code)]
    fn dump(&self, pipe: &Pipeline) {
        // dbg!(&self.lsq);
        dbg!(&self.reg_file);
        dbg!(&self.reservation_station);
        dbg!(&self.rob);
        dbg!(&self.execution_units);
        dbg!(pipe);
    }

    fn stage_fetch_decode(&mut self, pipe: &Pipeline) -> stages::FetchDecode {
        let tag = Tag::from(self.cycles); // Assumes we only fetch 1 instruction per cycle.

        // Branch prediction
        let pc = pipe.fetch_decode.next_pc;
        let inst = self.prog.fetch(pc).cloned().unwrap_or(Inst::Halt);
        let next_pc = match &inst {
            Inst::BranchIfEqual(_, _, tgt)
            | Inst::BranchIfNotEqual(_, _, tgt)
            | Inst::BranchIfLess(_, _, tgt)
            | Inst::BranchIfGreaterEqual(_, _, tgt) => {
                let taken_pc = self.prog.labels[tgt];
                let not_taken_pc = pc + 1;
                let predict_taken = self.branch_predictor.predict_taken(pc, taken_pc);

                self.reg_file
                    .begin_predict(tag, predict_taken, taken_pc, not_taken_pc);

                if predict_taken {
                    taken_pc
                } else {
                    not_taken_pc
                }
            }
            Inst::Jump(ref tgt) => self.prog.labels[tgt],
            _ => pc + 1,
        };

        stages::FetchDecode {
            inst: Some(Tagged { inst, tag }),
            next_pc,
        }
    }

    fn stage_rename(&mut self, pipe: &Pipeline) -> stages::Rename {
        let Tagged { inst, tag } = match &pipe.fetch_decode.inst {
            Some(inst) => inst.clone(),
            None => return stages::Rename::default(),
        };

        if self.rob.is_full() || self.reservation_station.len() == self.rs_max {
            return stages::Rename {
                inst: None,
                should_stall: true,
            };
        }

        if inst.is_mem_access() && !self.lsq.has_space(&inst) {
            return stages::Rename {
                inst: None,
                should_stall: true,
            };
        }

        if let Some(renamed_inst) = self.reg_file.perform_rename(inst.clone()) {
            assert_eq!(self.rob.try_push(tag, inst), None);
            assert_eq!(
                self.reservation_station.insert(tag, renamed_inst.clone()),
                None
            );

            if renamed_inst.is_mem_access() {
                self.lsq.insert_access(renamed_inst.clone(), tag);
            }

            stages::Rename {
                inst: Some(renamed_inst),
                should_stall: false,
            }
        } else {
            stages::Rename {
                inst: None,
                should_stall: true,
            }
        }
    }

    fn stage_issue(&mut self, _pipe: &Pipeline) -> stages::Issue {
        // Try to issue more instructions to reservation stations.
        let mut remove_tags = vec![];

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

        stages::Issue
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
                let mut next_fetch = None;

                match &inst {
                    Inst::Add(dst, _, _)
                    | Inst::Rem(dst, _, _)
                    | Inst::AddImm(dst, _, _)
                    | Inst::AndImm(dst, _, _)
                    | Inst::ShiftLeftLogicalImm(dst, _, _)
                    | Inst::LoadWord(dst, _) => {
                        if dst.arch != ArchReg::Zero {
                            self.reg_file.set_phys_active(dst.phys, result.val);
                        }
                    }
                    Inst::BranchIfEqual(_, _, _)
                    | Inst::BranchIfNotEqual(_, _, _)
                    | Inst::BranchIfGreaterEqual(_, _, _) => {
                        let taken = result.val == 1;
                        let predicted_taken = self.reg_file.was_predicted_taken(tag);

                        if let Some(next_pc) =
                            self.reg_file.end_predict(tag, taken, predicted_taken)
                        {
                            // Flush
                            self.kill_tags_after(tag);
                            next_fetch = Some(next_pc);
                        }
                    }
                    Inst::Jump(_) | Inst::StoreWord(_, _) | Inst::Halt => (),
                    _ => unimplemented!("{:?}", inst),
                }

                self.rob.mark_complete(tag);

                return stages::Writeback {
                    inst: Some(Tagged { tag, inst }),
                    next_fetch,
                };
            }
        }

        stages::Writeback {
            inst: None,
            next_fetch: None,
        }
    }

    // Commit instructions from the ROB to architectural state.
    fn stage_commit(&mut self, _pipe: &Pipeline) -> stages::Commit {
        let tagged = self.rob.try_pop();

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
            Inst::Add(dst, _, _)
            | Inst::Rem(dst, _, _)
            | Inst::AddImm(dst, _, _)
            | Inst::AndImm(dst, _, _)
            | Inst::ShiftLeftLogicalImm(dst, _, _)
            | Inst::LoadWord(dst, _) => {
                if dst != ArchReg::Zero {
                    self.reg_file.release_phys();
                }

                if inst.is_mem_access() {
                    self.lsq.release_load(tag);
                }
            }
            Inst::StoreWord(_, _) => self.lsq.submit_store(tag, &self.reg_file, &mut self.mem),
            Inst::Jump(_)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _) => (),
            _ => unimplemented!("{:?}", inst),
        }

        stages::Commit {
            inst: Some(inst),
            should_halt: false,
            retire: true,
        }
    }

    fn kill_tags_after(&mut self, tag: Tag) {
        self.reservation_station.retain(|&t, _| t <= tag);

        for eu in &mut self.execution_units {
            eu.kill_tags_after(tag);
        }

        self.lsq.kill_tags_after(tag);
        self.rob.kill_tags_after(tag);
        self.reg_file.kill_tags_after(tag);
    }
}
