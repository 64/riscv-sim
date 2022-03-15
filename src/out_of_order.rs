use std::collections::HashMap;

use crate::{
    cpu::{Cpu, ExecResult},
    execution_unit::{EuResult, EuType, ExecutionUnit},
    inst::{ArchReg, Inst, ReadyInst, RenamedInst, WithTag},
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
        pub result: EuResult,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Commit {
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Clone)]
pub struct Scheduler {
    rs: Queue<RenamedInst>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            rs: Queue::new(10), // Number of entries in the RS buffer. Fully unified.
        }
    }

    fn rename(&mut self, inst: Inst) -> RenamedInst {
        inst.map_src_reg(|r| WithTag::Valid(0))
    }

    fn schedule(&mut self, eus: &mut [ExecutionUnit], inst: Inst) -> bool {
        if self.rs.is_full() {
            return true;
        }

        let renamed_inst = self.rename(inst);
        assert!(self.rs.try_push(renamed_inst).is_none());

        // Try to issue more instructions to reservation stations.
        let mut remove_indices = vec![];

        for (i, ready_inst) in self
            .rs
            .iter()
            .enumerate()
            .filter_map(|(i, inst)| inst.get_ready().map(|inst| (i, inst)))
        {
            if let Some(eu) = eus.iter_mut().find(|eu| eu.can_execute(&ready_inst)) {
                remove_indices.push(i);
                eu.begin_execute(ready_inst);
            }
        }

        for i in remove_indices {
            self.rs.remove(i);
        }

        false
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
    scheduler: Scheduler,
    rat: RegisterAliasTable,
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
            scheduler: Scheduler::new(),
            rat: RegisterAliasTable::new(regs),
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut cycles = 0;
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
                    cycles_taken: cycles,
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

impl OutOfOrder {
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
        let should_stall = self.scheduler.schedule(&mut self.execution_units, inst);

        stages::DecodeIssue { should_stall }
    }

    fn stage_execute(&mut self, pipe: &Pipeline) -> stages::Execute {
        // Advance execution of all the execution units.
        for eu in &mut self.execution_units {
            eu.advance();
        }

        stages::Execute
    }

    fn stage_writeback(&mut self, pipe: &Pipeline) -> stages::Writeback {
        // Take output of the execution units and write into the ROB.

        for eu in &mut self.execution_units {
            if let Some((completed, result)) = eu.take_complete() {
                // Only allow writeback of 1 instruction per cycle, for now.
                return stages::Writeback {
                    inst: Some(completed),
                    result,
                };
            }
        }

        stages::Writeback {
            inst: None,
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
            Inst::AddImm(dst, _, _) => self.rat.set_value(dst, pipe.writeback.result.val),
            _ => todo!(),
        }

        stages::Commit {
            should_halt: false,
            retire: true,
        }
    }
}
