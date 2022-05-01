use crate::{
    cpu::Stats,
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
    pub eu_type: EuType,
    pub utilisation: u64,
    completed_inst: Option<(Tagged<ExecutedInst>, EuResult)>,
    executing_insts: Vec<(Tagged<ReadyInst>, CyclesTaken)>,
}

// TODO: get rid of begin_inst

impl ExecutionUnit {
    pub fn new(eu_type: EuType) -> Self {
        Self {
            eu_type,
            utilisation: 0,
            completed_inst: Default::default(),
            executing_insts: Default::default(),
        }
    }

    pub fn can_execute(&self, inst: &ReadyInst) -> bool {
        self.eu_type == inst.eu_type() && !self.was_utilised()
    }

    pub fn begin_execute(&mut self, inst: ReadyInst, tag: Tag) {
        debug_assert!(self.can_execute(&inst));
        self.executing_insts.push((Tagged { tag, inst }, 0));
    }

    pub fn was_utilised(&self) -> bool {
        self.executing_insts
            .last()
            .map(|e| e.1 == 0)
            .unwrap_or(false)
    }

    pub fn advance(&mut self, mem: &mut MemoryHierarchy, stats: &mut Stats) {
        let mut deleted_idx = None;

        if self.was_utilised() {
            self.utilisation += 1;
        }

        for (i, (Tagged { tag, inst }, cycles)) in self.executing_insts.iter_mut().enumerate() {
            let is_done = if inst.is_mem_access() {
                mem.access_complete(*tag, inst.access_addr(), stats)
            } else {
                *cycles + 1 >= inst.latency()
            };

            if is_done && self.completed_inst.is_none() {
                if inst.is_mem_access() {
                    mem.finish_access(*tag, inst.access_addr());
                }

                let res = ExecutionUnit::compute_result(inst, mem);
                deleted_idx = Some(i);
                self.completed_inst = Some((
                    Tagged {
                        tag: *tag,
                        inst: inst.clone().executed(),
                    },
                    res,
                ));
            } else {
                *cycles += 1;
            }
        }

        deleted_idx.map(|i| self.executing_insts.remove(i));
    }

    pub fn take_complete(&mut self) -> Option<(Tagged<ExecutedInst>, EuResult)> {
        self.completed_inst.take()
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.executing_insts
            .retain(|(Tagged { tag: t, .. }, _)| *t <= tag);

        if let Some((tagged, _)) = &self.completed_inst {
            if tagged.tag > tag {
                self.completed_inst = None;
            }
        }
    }

    fn compute_result(inst: &ReadyInst, mem: &mut MemoryHierarchy) -> EuResult {
        let val = match inst {
            Inst::Add(_, src0, src1) => src0.wrapping_add(*src1),
            Inst::And(_, src0, src1) => src0 & *src1,
            Inst::Or(_, src0, src1) => src0 | *src1,
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
            Inst::DivU(_, src0, src1) => {
                if *src1 == 0 {
                    u32::MAX
                } else {
                    *src0 / *src1
                }
            }
            Inst::ShiftLeftLogicalImm(_, src, imm) => src.wrapping_shl(imm.0),
            Inst::SetLessThanImmU(_, src, imm) => (src < &imm.0).into(),
            Inst::JumpAndLink(_, imm) => imm.0,
            Inst::BranchIfEqual(src0, src1, _) => (src0 == src1).into(),
            Inst::BranchIfNotEqual(src0, src1, _) => (src0 != src1).into(),
            Inst::BranchIfGreaterEqualU(src0, src1, _) => (src0 >= src1).into(),
            Inst::BranchIfGreaterEqual(src0, src1, _) => {
                let a = i32::from_le_bytes(src0.to_le_bytes());
                let b = i32::from_le_bytes(src1.to_le_bytes());
                (a >= b).into()
            }
            Inst::BranchIfLess(src0, src1, _) => {
                let a = i32::from_le_bytes(src0.to_le_bytes());
                let b = i32::from_le_bytes(src1.to_le_bytes());
                (a < b).into()
            }
            Inst::BranchIfLessU(src0, src1, _) => (src0 < src1).into(),
            Inst::Halt => 0,
            Inst::LoadByteU(_, src) => mem.main.readbu(src.compute_addr()),
            Inst::LoadWord(_, src) => mem.main.readw(src.compute_addr()),
            x if x.is_store() => 0, // Stores are handled by LSQ upon retire.
            _ => unimplemented!("{:?}", inst),
        };

        EuResult { val }
    }
}
