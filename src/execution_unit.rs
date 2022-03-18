use crate::{
    inst::{ExecutedInst, Inst, ReadyInst, Tag},
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
    begin_inst: Option<(ReadyInst, Tag)>,
    executing_inst: Option<(ReadyInst, Tag, CyclesTaken)>,
    completed_inst: Option<(ExecutedInst, Tag, EuResult)>,
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
        self.begin_inst = Some((inst, tag));
    }

    pub fn advance(&mut self, mem: &mut Memory) {
        if let Some((inst, tag, cycles)) = self.executing_inst.take() {
            if cycles + 1 >= inst.latency() && self.completed_inst.is_none() {
                let res = self.compute_result(&inst, mem);
                self.completed_inst = Some((inst.executed(), tag, res));
            } else {
                // Increment cycles, and carry on.
                self.executing_inst = Some((inst, tag, cycles + 1));
            }
        } else if let Some((inst, tag)) = self.begin_inst.take() {
            self.executing_inst = Some((inst, tag, 0));
        }
    }

    pub fn take_complete(&mut self) -> Option<(ExecutedInst, Tag, EuResult)> {
        self.completed_inst.take()
    }

    fn compute_result(&self, inst: &ReadyInst, mem: &mut Memory) -> EuResult {
        let val = match inst {
            Inst::AddImm(_, src, imm) => src + imm.0,
            Inst::StoreWord(val, dst) => {
                mem.writew(dst.compute_addr(), *val);
                0
            }
            Inst::LoadWord(_, src) => mem.readw(src.compute_addr()),
            Inst::Halt => 0,
            _ => unimplemented!("{:?}", inst),
        };

        EuResult { val }
    }
}
