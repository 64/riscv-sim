use crate::{
    inst::{Inst, Tag, Tagged},
    queue::Queue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RobStatus {
    Executing,
    Executed,
}

#[derive(Debug, Clone)]
pub struct RobEntry {
    tag: Tag,
    inst: Inst,
    status: RobStatus,
}

#[derive(Debug, Clone)]
pub struct ReorderBuffer {
    rob: Queue<RobEntry>,
}

impl ReorderBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            rob: Queue::new(capacity),
        }
    }

    pub fn is_full(&self) -> bool {
        self.rob.is_full()
    }

    pub fn last_is_halt(&self) -> bool {
        self.rob
            .back()
            .map(|ent| ent.inst == Inst::Halt)
            .unwrap_or(false)
    }

    #[must_use]
    pub fn try_push(&mut self, tag: Tag, inst: Inst) -> Option<Inst> {
        self.rob
            .try_push(RobEntry {
                tag,
                inst,
                status: RobStatus::Executing,
            })
            .map(|ent| ent.inst)
    }

    pub fn try_pop(&mut self) -> Option<Tagged<Inst>> {
        if self
            .rob
            .front()
            .map(|ent| ent.status == RobStatus::Executed)
            .unwrap_or(false)
        {
            self.rob.try_pop().map(|ent| Tagged {
                tag: ent.tag,
                inst: ent.inst,
            })
        } else {
            None
        }
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.rob.retain(|ent| ent.tag <= tag);
    }

    pub fn mark_complete(&mut self, tag: Tag) {
        let ent = self
            .rob
            .iter_mut()
            .find(|ent| ent.tag == tag)
            .expect("no entry found in ROB");

        ent.status = RobStatus::Executed;
    }
}
