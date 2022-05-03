use std::ops::Range;

use crate::{
    inst::{Inst, RenamedInst, Tag, Tagged},
    mem::MemoryHierarchy,
    queue::Queue,
    regs::RegFile,
};

#[derive(Debug, Clone)]
pub struct Store {
    tagged: Tagged<RenamedInst>,
    address: Option<Range<u32>>,
}

#[derive(Debug, Clone)]
pub struct Load {
    tagged: Tagged<RenamedInst>,
    address: Option<Range<u32>>,
    completed: bool,
}

#[derive(Debug, Clone)]
pub struct LoadStoreQueue {
    loads: Queue<Load>,
    stores: Queue<Store>,
}

pub const MEM_SPECULATION: bool = true;

impl Store {
    pub fn new(tagged: Tagged<RenamedInst>) -> Self {
        Self {
            tagged,
            address: None,
        }
    }
}

impl Load {
    pub fn new(tagged: Tagged<RenamedInst>) -> Self {
        Self {
            tagged,
            address: None,
            completed: false,
        }
    }
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
            self.loads.try_push(Load::new(entry)).map(|_| ())
        } else if entry.inst.is_store() {
            self.stores.try_push(Store::new(entry)).map(|_| ())
        } else {
            unreachable!()
        };

        assert!(res.is_none(), "no space in LSQ");
    }

    pub fn can_execute_load(&mut self, tag: Tag, load_addr: Range<u32>) -> bool {
        if MEM_SPECULATION {
            // Insert the load_addr.
            let load = self
                .loads
                .iter_mut()
                .find(|l| l.tagged.tag == tag)
                .expect("no load");
            load.address = Some(load_addr.clone());

            // Optimistically return yes, unless we know the answer is definitely no.
            self.stores.iter().filter(|s| s.tagged.tag < tag).all(|s| {
                if let Some(store_addr) = &s.address {
                    Self::ranges_disjoint(store_addr, &load_addr)
                } else {
                    // Begin a speculation
                    true
                }
            })
        } else {
            // Stores are entered into the LSQ in order.
            // Here we enforce that the next store must occur after the designated load
            self.stores
                .front()
                .map(|s| s.tagged.tag > tag)
                .unwrap_or(true)
        }
    }

    fn ranges_disjoint(a: &Range<u32>, b: &Range<u32>) -> bool {
        a.start <= b.end && a.end <= b.start
    }

    pub fn store_addr_known(&mut self, tag: Tag, store_addr: Range<u32>) {
        if MEM_SPECULATION {
            let store = self
                .stores
                .iter_mut()
                .find(|s| s.tagged.tag == tag)
                .expect("no store");
            store.address = Some(store_addr.clone());

            // todo dont repeat this
            // Check if any later loads overlap us
            let store_tag = store.tagged.tag;
            let loads_to_kill = self.loads
                .iter()
                .filter(|l| l.tagged.tag > store_tag && l.completed && l.address.is_some())
                .filter(|l| !Self::ranges_disjoint(l.address.as_ref().unwrap(), &store_addr))
                .map(|l| l.tagged.tag)
                .collect::<Vec<_>>();

            assert!(loads_to_kill.is_empty());
        }
    }

    pub fn submit_store(&mut self, tag: Tag, rf: &RegFile, mem: &mut MemoryHierarchy) {
        let store = self.stores.try_pop().unwrap();
        debug_assert_eq!(store.tagged.tag, tag);

        match store
            .tagged
            .inst
            .get_ready(rf)
            .expect("store committed when not ready")
        {
            Inst::StoreByte(val, dst) => {
                mem.main.writeb(dst.compute_addr(), val);
            }
            Inst::StoreWord(val, dst) => {
                mem.main.writew(dst.compute_addr(), val);
            }
            _ => unimplemented!("{:?}", store.tagged.inst),
        }
    }

    pub fn load_complete(&mut self, tag: Tag) {
        let load = self.loads.iter_mut().find(|l| l.tagged.tag == tag).expect("no load found");
        load.completed = true;
    }

    pub fn release_load(&mut self, tag: Tag) {
        self.loads.retain(|l| l.tagged.tag != tag);
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.loads.retain(|ent| ent.tagged.tag <= tag);
        self.stores.retain(|ent| ent.tagged.tag <= tag);
    }
}
