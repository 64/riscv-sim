use crate::{
    inst::{Inst, Tag},
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

    pub fn is_earliest_mem_access(&self, tag: Tag) -> bool {
        self.rob
            .iter()
            .find(|ent| ent.inst.is_mem_access())
            .map(|ent| ent.tag)
            == Some(tag)
    }

    pub fn try_push(&mut self, tag: Tag, inst: Inst) -> Option<Inst> {
        self.rob
            .try_push(RobEntry {
                tag,
                inst,
                status: RobStatus::Executing,
            })
            .map(|ent| ent.inst)
    }

    pub fn try_pop(&mut self) -> Option<Inst> {
        if self
            .rob
            .front()
            .map(|ent| ent.status == RobStatus::Executed)
            .unwrap_or(false)
        {
            self.rob.try_pop().map(|ent| ent.inst)
        } else {
            None
        }
    }

    pub fn mark_complete(&mut self, tag: Tag) {
        let rob_entry = self
            .rob
            .iter_mut()
            .find(|ent| ent.tag == tag)
            .expect("no entry found in ROB");
        rob_entry.status = RobStatus::Executed;
    }
}
