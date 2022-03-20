use crate::{
    inst::{ExecutedInst, Inst, ReadyInst, Tag, Tagged},
    mem::Memory,
};

#[derive(Debug, Clone, Default)]
pub struct EuResult {
    pub val: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EuType {
    ALU,
    LoadStore,
    Special, // Halt and such.
}

type CyclesTaken = u64;

#[derive(Debug, Clone)]
pub struct ExecutionUnit {
    eu_type: EuType,
    begin_inst: Option<Tagged<ReadyInst>>,
    executing_inst: Option<(Tagged<ReadyInst>, CyclesTaken)>,
    completed_inst: Option<(Tagged<ExecutedInst>, EuResult)>,
}

impl ExecutionUnit {
    pub fn new(eu_type: EuType) -> Self {
        Self {
            eu_type,
            begin_inst: Default::default(),
            executing_inst: Default::default(),
            completed_inst: Default::default(),
        }
    }

    pub fn can_execute(&self, inst: &ReadyInst) -> bool {
        self.eu_type == inst.eu_type() && self.begin_inst.is_none() && self.executing_inst.is_none()
    }

    pub fn begin_execute(&mut self, inst: ReadyInst, tag: Tag) {
        debug_assert!(self.can_execute(&inst));
        self.begin_inst = Some(Tagged { tag, inst });
    }

    pub fn advance(&mut self, mem: &mut Memory) {
        if let Some((Tagged { tag, inst }, cycles)) = self.executing_inst.take() {
            if cycles + 1 >= inst.latency() && self.completed_inst.is_none() {
                let res = self.compute_result(&inst, mem);
                self.completed_inst = Some((
                    Tagged {
                        tag,
                        inst: inst.executed(),
                    },
                    res,
                ));
            } else {
                // Increment cycles, and carry on.
                self.executing_inst = Some((Tagged { tag, inst }, cycles + 1));
            }
        } else if let Some(tagged) = self.begin_inst.take() {
            self.executing_inst = Some((tagged, 0));
        }
    }

    pub fn take_complete(&mut self) -> Option<(Tagged<ExecutedInst>, EuResult)> {
        self.completed_inst.take()
    }

    fn compute_result(&self, inst: &ReadyInst, mem: &mut Memory) -> EuResult {
        let val = match inst {
            Inst::AddImm(_, src, imm) => src.wrapping_add(imm.0),
            Inst::LoadWord(_, src) => mem.readw(src.compute_addr()),
            Inst::BranchIfEqual(src0, src1, _) => (src0 == src1).into(),
            Inst::BranchIfNotEqual(src0, src1, _) => (src0 != src1).into(),
            Inst::BranchIfGreaterEqual(src0, src1, _) => (src0 >= src1).into(),
            Inst::Halt => 0,
            x if x.is_store() => 0, // Stores are handled by LSQ upon retire.
            _ => unimplemented!("{:?}", inst),
        };

        EuResult { val }
    }
}
