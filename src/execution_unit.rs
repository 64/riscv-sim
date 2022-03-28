use crate::{
    inst::{ExecutedInst, Inst, ReadyInst, Tag, Tagged},
    mem::MemoryHierarchy,
};

#[derive(Debug, Clone, Default)]
pub struct EuResult {
    pub val: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EuType {
    Alu,
    LoadStore,
    Branch,
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

// TODO: get rid of begin_inst

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

    pub fn advance(&mut self, mem: &mut MemoryHierarchy) {
        if let Some((Tagged { tag, inst }, cycles)) = self.executing_inst.take() {
            let is_done = if inst.is_mem_access() {
                mem.access_complete(tag, inst.access_addr())
            } else {
                cycles + 1 >= inst.latency()
            };

            if is_done && self.completed_inst.is_none() {
                if inst.is_mem_access() {
                    mem.finish_access(tag, inst.access_addr());
                }

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

    pub fn kill_tags_after(&mut self, tag: Tag) {
        if let Some(tagged) = &self.begin_inst {
            if tagged.tag > tag {
                self.begin_inst = None;
            }
        }
        if let Some((tagged, _)) = &self.executing_inst {
            if tagged.tag > tag {
                self.executing_inst = None;
            }
        }
        if let Some((tagged, _)) = &self.completed_inst {
            if tagged.tag > tag {
                self.completed_inst = None;
            }
        }
    }

    fn compute_result(&self, inst: &ReadyInst, mem: &mut MemoryHierarchy) -> EuResult {
        let val = match inst {
            Inst::Add(_, src0, src1) => src0.wrapping_add(*src1),
            Inst::Sub(_, src0, src1) => src0.wrapping_sub(*src1),
            Inst::AddImm(_, src, imm) => src.wrapping_add(imm.0),
            Inst::AndImm(_, src, imm) => src & imm.0,
            Inst::Mul(_, src0, src1) => src0.wrapping_mul(*src1),
            Inst::Rem(_, src0, src1) => {
                if *src1 == 0 {
                    *src0
                } else {
                    *src0 % *src1
                }
            }
            Inst::ShiftLeftLogicalImm(_, src, imm) => src.wrapping_shl(imm.0),
            Inst::SetLessThanImmU(_, src, imm) => (src < &imm.0).into(),
            Inst::BranchIfEqual(src0, src1, _) => (src0 == src1).into(),
            Inst::BranchIfNotEqual(src0, src1, _) => (src0 != src1).into(),
            Inst::BranchIfGreaterEqual(src0, src1, _) => (src0 >= src1).into(),
            Inst::Jump(_) | Inst::Halt => 0,
            Inst::LoadWord(_, src) => mem.main.readw(src.compute_addr()),
            x if x.is_store() => 0, // Stores are handled by LSQ upon retire.
            _ => unimplemented!("{:?}", inst),
        };

        EuResult { val }
    }
}
