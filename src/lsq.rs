use std::ops::Range;

use crate::{
    inst::{AbsPc, Inst, RenamedInst, Tag, Tagged},
    mem::MemoryHierarchy,
    queue::Queue,
    regs::RegFile,
};

#[derive(Debug, Clone)]
pub struct Store {
    tagged: Tagged<RenamedInst>,
    address: Option<Range<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadStatus {
    NotExecuting,
    InFlight,
    WrittenBack,
}

#[derive(Debug, Clone)]
pub struct Load {
    tagged: Tagged<RenamedInst>,
    address: Option<Range<u32>>,
    status: LoadStatus,
}

#[derive(Debug, Clone)]
pub struct LoadStoreQueue {
    loads: Queue<Load>,
    stores: Queue<Store>,
}

pub const MEM_SPECULATION: bool = false;

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
            status: LoadStatus::NotExecuting,
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

    pub fn begin_execute_load(&mut self, load: Tag) {
        let load = self
            .loads
            .iter_mut()
            .find(|l| l.tagged.tag == load)
            .expect("no load");
        debug_assert!(load.status == LoadStatus::NotExecuting);
        load.status = LoadStatus::InFlight;
    }

    pub fn can_execute_load(
        &mut self,
        tag: Tag,
        _load_pc: AbsPc,
        load_addr: Range<u32>,
        _reg_file: &mut RegFile,
    ) -> bool {
        if MEM_SPECULATION {
            // Insert the load_addr.
            let load = self
                .loads
                .iter_mut()
                .find(|l| l.tagged.tag == tag)
                .expect("no load");
            load.address = Some(load_addr.clone());

            // Optimistically return yes, unless we know the answer is definitely no.
            !self.stores.iter().filter(|s| s.tagged.tag < tag).any(|s| {
                if let Some(store_addr) = &s.address {
                    // dbg!(&self);
                    Self::ranges_overlap(store_addr, &load_addr)
                } else {
                    // Begin a speculation
                    // println!("BEGIN SPECULATE {:?} at {:?}", tag, load_pc);
                    // reg_file.begin_predict_mem(tag, load_pc);
                    false
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

    fn ranges_overlap(a: &Range<u32>, b: &Range<u32>) -> bool {
        b.contains(&a.start) || a.contains(&b.start)
    }

    pub fn store_addr_known(
        &mut self,
        store_tag: Tag,
        store_addr: Range<u32>,
        _reg_file: &mut RegFile,
    ) -> (Vec<Tag>, Vec<Tag>) {
        if MEM_SPECULATION {
            let store = self
                .stores
                .iter_mut()
                .find(|s| s.tagged.tag == store_tag)
                .expect("no store");
            store.address = Some(store_addr.clone());

            // todo dont repeat this
            // Check if any later loads overlap us
            let loads_to_kill = self
                .loads
                .iter()
                .filter(|l| {
                    l.tagged.tag > store_tag
                        && l.status != LoadStatus::NotExecuting
                        && l.address.is_some()
                })
                .filter(|l| Self::ranges_overlap(l.address.as_ref().unwrap(), &store_addr))
                .collect::<Vec<&Load>>();

            // if !loads_to_kill.is_empty() {
            // dbg!(store_tag);
            // dbg!(&self);
            // dbg!(&loads_to_kill);
            // }

            // assert!(loads_to_kill.is_empty());
            let eu_kills = loads_to_kill
                .iter()
                .filter(|l| l.status == LoadStatus::InFlight)
                .map(|l| l.tagged.tag)
                .collect();
            let mispredicts = loads_to_kill
                .iter()
                .filter(|l| l.status == LoadStatus::WrittenBack)
                .map(|l| l.tagged.tag)
                .collect();
            (eu_kills, mispredicts)
        } else {
            (Vec::new(), Vec::new())
        }
    }

    pub fn kill_inflight(&mut self, load: Tag) {
        let load = self
            .loads
            .iter_mut()
            .find(|l| l.tagged.tag == load)
            .expect("no load found");

        load.status = LoadStatus::NotExecuting;
    }

    pub fn commit_store(&mut self, tag: Tag, rf: &RegFile, mem: &mut MemoryHierarchy) {
        let store = self.stores.try_pop().unwrap();
        debug_assert_eq!(store.tagged.tag, tag);
        // println!("COMMITTED STORE {:?}", tag);

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

    pub fn writeback_load(&mut self, tag: Tag) {
        let load = self
            .loads
            .iter_mut()
            .find(|l| l.tagged.tag == tag)
            .expect("no load found");
        // println!("WRITTEN BACK LOAD {:?}", tag);
        debug_assert!(load.status == LoadStatus::InFlight);
        load.status = LoadStatus::WrittenBack;
    }

    pub fn release_load(&mut self, tag: Tag) {
        self.loads.retain(|l| l.tagged.tag != tag);
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        self.loads.retain(|ent| ent.tagged.tag <= tag);
        self.stores.retain(|ent| ent.tagged.tag <= tag);
    }
}
