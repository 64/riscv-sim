use crate::{
    inst::{ReadyInst, RenamedInst, Tag},
    regs::RegFile,
};

#[derive(Debug, Clone)]
pub struct ReservationStation {
    waiting: Vec<(Tag, RenamedInst)>,
    ready: Vec<(Tag, ReadyInst)>,
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
        self.waiting.push((tag, inst));
    }

    // TODO: This interface (get_ready, pop_ready) is a bit janky
    pub fn get_ready(&mut self, reg_file: &RegFile) -> impl Iterator<Item = &(Tag, ReadyInst)> {
        self.waiting.retain(
            |(tag, renamed_inst)| match renamed_inst.get_ready(reg_file) {
                Some(ready_inst) => {
                    let pos = self
                        .ready
                        .binary_search_by_key(&tag, |(t, _)| t)
                        .unwrap_err();
                    self.ready.insert(pos, (*tag, ready_inst));
                    false
                }
                None => true,
            },
        );

        self.ready.iter()
    }

    pub fn pop_ready(&mut self, tag: Tag) {
        let pos = self.ready.iter().position(|&(t, _)| t == tag).unwrap();
        self.ready.remove(pos);
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.waiting.retain(|&(t, _)| t <= tag);
        self.ready.retain(|&(t, _)| t <= tag);
    }
}
