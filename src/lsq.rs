use crate::{
    inst::{Inst, RenamedInst, Tag, Tagged},
    mem::Memory,
    queue::Queue,
    regs::RegFile,
};

type Load = Tagged<RenamedInst>;
type Store = Tagged<RenamedInst>;

#[derive(Debug, Clone)]
pub struct LoadStoreQueue {
    loads: Queue<Load>,
    stores: Queue<Store>,
}

impl LoadStoreQueue {
    pub fn new(load_capacity: usize, store_capacity: usize) -> Self {
        Self {
            loads: Queue::new(load_capacity),
            stores: Queue::new(store_capacity),
        }
    }

    pub fn has_space(&self, inst: &Inst) -> bool {
        if inst.is_load() {
            !self.loads.is_full()
        } else if inst.is_store() {
            !self.stores.is_full()
        } else {
            true
        }
    }

    pub fn insert_access(&mut self, inst: RenamedInst, tag: Tag) {
        let entry = Tagged { tag, inst };

        let res = if entry.inst.is_load() {
            self.loads.try_push(entry)
        } else if entry.inst.is_store() {
            self.stores.try_push(entry)
        } else {
            unreachable!()
        };

        assert!(res.is_none(), "no space in LSQ");
    }

    pub fn can_execute_load(&self, tag: Tag) -> bool {
        self.stores
            .front()
            .map(|s| s.tag > tag) // The next store must occur after the designated load
            .unwrap_or(true)
    }

    pub fn submit_store(&mut self, tag: Tag, rf: &RegFile, mem: &mut Memory) {
        let store = self.stores.try_pop().unwrap();
        debug_assert_eq!(store.tag, tag);

        match store
            .inst
            .get_ready(rf)
            .expect("store committed when not ready")
        {
            Inst::StoreWord(val, dst) => {
                mem.writew(dst.compute_addr(), val);
            }
            _ => unreachable!(),
        }
    }

    pub fn release_load(&mut self, tag: Tag) {
        self.loads.retain(|l| l.tag != tag);
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.loads.retain(|ent| ent.tag <= tag);
        self.stores.retain(|ent| ent.tag <= tag);
    }
}
