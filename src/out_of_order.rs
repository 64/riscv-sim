use std::collections::HashMap;

use crate::{
    cpu::{Cpu, ExecResult},
    execution_unit::{EuResult, EuType, ExecutionUnit},
    inst::{ArchReg, Inst, ReadyInst, RenamedInst, Tag, ValueOrTag},
    mem::Memory,
    program::Program,
    queue::Queue,
    rat::RegisterAliasTable,
};

mod stages {
    use crate::inst::ExecutedInst;

    use super::*;

    #[derive(Debug, Clone, Default)]
    pub struct Fetch {
        pub inst: Option<Inst>,
        pub next_pc: u32,
    }

    #[derive(Debug, Clone, Default)]
    pub struct DecodeIssue {
        pub should_stall: bool,
        // pub inst: Option<Inst>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Execute;

    #[derive(Debug, Clone, Default)]
    pub struct Writeback {
        pub inst: Option<ExecutedInst>,
        pub tag: Tag,
        pub result: EuResult,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Commit {
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Clone, Default)]
pub struct Pipeline {
    fetch: stages::Fetch,
    decode_issue: stages::DecodeIssue,
    execute: stages::Execute,
    writeback: stages::Writeback,
    commit: stages::Commit,
}

#[derive(Debug, Clone)]
pub struct OutOfOrder {
    execution_units: Vec<ExecutionUnit>,
    mem: Memory,
    prog: Program,
    reservation_station: HashMap<Tag, RenamedInst>,
    frontend_rat: RegisterAliasTable,
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
            reservation_station: HashMap::new(),
            rs_max: 10,
            cycles: 0,
            frontend_rat: RegisterAliasTable::new(regs),
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut insts_retired = 0;
        let mut pipe = Pipeline::default();

        loop {
            let fetch = self.stage_fetch(&pipe);
            let decode_issue = self.stage_decode_issue(&pipe);
            let writeback = self.stage_writeback(&pipe);
            let execute = self.stage_execute(&pipe);
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
        dbg!(&self.frontend_rat);
        dbg!(&self.reservation_station);
        dbg!(&self.execution_units);
        dbg!(pipe);
    }

    fn rename_and_reserve(&mut self, inst: Inst) -> bool {
        let mut inst_dst_reg = None;
        let renamed_inst = inst.map_regs(
            |src_reg| self.frontend_rat.get(src_reg),
            |dst_reg| {
                let old_reg = inst_dst_reg.replace(dst_reg); // Disallow more than one dst reg.
                debug_assert_eq!(old_reg, None);
                dst_reg
            },
        );

        // Assumes we only issue 1 inst per cycle.
        let tag = Tag::from(self.cycles);

        if let Some(dst_reg) = inst_dst_reg {
            if self.frontend_rat.rename(dst_reg, tag) {
                // return true; // We need to stall here. TODO: ROB
            }
        }

        assert_eq!(self.reservation_station.insert(tag, renamed_inst), None);
        false
    }

    fn issue(&mut self, inst: Inst) -> bool {
        // Try to issue more instructions to reservation stations.
        let mut remove_tags = vec![];

        for (tag, ready_inst) in self
            .reservation_station
            .iter()
            .filter_map(|(&tag, inst)| inst.get_ready().map(|inst| (tag, inst)))
        {
            dbg!(tag, &inst);
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

        if self.reservation_station.len() == self.rs_max {
            return true;
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
        let should_stall = self.issue(inst);

        stages::DecodeIssue { should_stall }
    }

    fn stage_execute(&mut self, pipe: &Pipeline) -> stages::Execute {
        // Advance execution of all the execution units.
        for eu in &mut self.execution_units {
            eu.advance(&mut self.mem);
        }

        stages::Execute
    }

    fn stage_writeback(&mut self, pipe: &Pipeline) -> stages::Writeback {
        // Take output of the execution units and write into the ROB.

        for eu in &mut self.execution_units {
            if let Some((completed, tag, result)) = eu.take_complete() {
                // Only allow writeback of 1 instruction per cycle, for now.
                return stages::Writeback {
                    inst: Some(completed),
                    tag,
                    result,
                };
            }
        }

        stages::Writeback {
            inst: None,
            tag: 0,
            result: EuResult::default(),
        }
    }

    fn stage_commit(&mut self, pipe: &Pipeline) -> stages::Commit {
        // Commit instructions from the ROB to memory.
        let inst = match &pipe.writeback.inst {
            Some(Inst::Halt) => {
                return stages::Commit {
                    should_halt: true,
                    retire: false,
                }
            }
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        match inst {
            Inst::AddImm(dst, _, _) | Inst::LoadWord(dst, _) => {
                self.frontend_rat.set_value(dst, pipe.writeback.result.val)
            }
            Inst::StoreWord(_, _) => (),
            _ => unimplemented!("{:?}", inst),
        }

        // Broadcast tag of completed instruction to waiting instructions.
        for (_, waiting_inst) in self.reservation_station.iter_mut() {
            let completed_tag = pipe.writeback.tag;
            let completed_result = pipe.writeback.result.val;
            *waiting_inst = waiting_inst.clone().map_regs(
                |src_reg| {
                    if src_reg == ValueOrTag::Invalid(completed_tag) {
                        ValueOrTag::Valid(completed_result)
                    } else {
                        src_reg
                    }
                },
                |dst_reg| dst_reg,
            );
        }

        stages::Commit {
            should_halt: false,
            retire: true,
        }
    }
}
