use std::collections::HashMap;

use crate::{
    cpu::{Cpu, ExecResult},
    inst::{ArchReg, Inst},
    mem::Memory,
    program::Program,
    regs::RegSet,
};

mod stages {
    use super::*;

    #[derive(Debug, Clone, Default)]
    pub struct Fetch {
        pub inst: Option<Inst>,
        pub next_pc: u32,
    }

    #[derive(Debug, Clone, Default)]
    pub struct DecodeIssue {
        pub inst: Option<Inst>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Execute {
        pub inst: Option<Inst>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Writeback {
        pub inst: Option<Inst>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct Commit {
        pub inst: Option<Inst>,
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionUnit {
    pub executing_inst: Option<Inst>,
    pub completed_inst: Option<Inst>,
}

impl ExecutionUnit {
    fn advance(&self) {
        todo!();
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scheduler {}

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
    initial_regs: RegSet,
    execution_units: Vec<ExecutionUnit>,
    mem: Memory,
    prog: Program,
    scheduler: Scheduler,
}

impl Cpu for OutOfOrder {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        Self {
            mem,
            prog,
            initial_regs: RegSet::new(regs),
            scheduler: Scheduler::default(),
            execution_units: Vec::default(),
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

            pipe = Pipeline {
                fetch,
                decode_issue,
                execute,
                writeback,
                commit,
            };

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

        // 1. Perform register renaming.
        // 2. Give instruction to the scheduler to be placed into an execution unit.
        // self.scheduler.schedule(renamed_inst);

        stages::DecodeIssue { inst: Some(inst) }
    }

    fn stage_execute(&mut self, pipe: &Pipeline) -> stages::Execute {
        // Advance execution of all the execution units.

        for eu in &self.execution_units {
            eu.advance();
        }

        todo!()
    }

    fn stage_writeback(&mut self, pipe: &Pipeline) -> stages::Writeback {
        // Take output of the execution units and write into the ROB.

        todo!()
    }

    fn stage_commit(&mut self, pipe: &Pipeline) -> stages::Commit {
        // Commit instructions from the ROB to memory.

        todo!()
    }
}
