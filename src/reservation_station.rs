use std::collections::BTreeMap;

use crate::{
    inst::{ReadyInst, RenamedInst, Tag},
    regs::RegFile,
};

#[derive(Debug, Clone)]
pub struct ReservationStation {
    waiting: BTreeMap<Tag, RenamedInst>,
    ready: BTreeMap<Tag, ReadyInst>,
    capacity: usize,
}

impl ReservationStation {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            waiting: Default::default(),
            ready: Default::default(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.waiting.len() + self.ready.len() >= self.capacity
    }

    pub fn insert(&mut self, tag: Tag, inst: RenamedInst) {
        debug_assert!(!self.is_full());
        self.waiting.insert(tag, inst);
    }

    pub fn get_ready(&mut self, reg_file: &RegFile) -> impl Iterator<Item = (&Tag, &ReadyInst)> {
        let mut remove_tags = vec![];

        for (&tag, renamed_inst) in &self.waiting {
            if let Some(ready_inst) = renamed_inst.get_ready(reg_file) {
                self.ready.insert(tag, ready_inst);
                remove_tags.push(tag);
            }
        }

        for tag in remove_tags {
            self.waiting.remove(&tag);
        }

        self.ready.iter()
    }

    pub fn pop_ready(&mut self, tag: Tag) {
        self.ready.remove(&tag);
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.waiting.retain(|&t, _| t <= tag);
        self.ready.retain(|&t, _| t <= tag);
    }
}
