#![allow(dead_code, unused)]

use associative_cache::*;
use hashbrown::HashMap;

use crate::{inst::AbsPc, util::CacheCapacity};

#[derive(Debug, Default)]
pub struct BranchTargetBuffer {
    cache: AssociativeCache<
        AbsPc,
        WithLruTimestamp<AbsPc>,
        CacheCapacity<512>,
        HashFourWay,
        LruReplacement,
    >,
}

impl Clone for BranchTargetBuffer {
    fn clone(&self) -> Self {
        Default::default()
    }
}

impl BranchTargetBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: AssociativeCache::default(),
        }
    }

    pub fn get(&mut self, pc: AbsPc) -> Option<AbsPc> {
        self.cache
            .get(&pc)
            .cloned()
            .map(|ent| WithLruTimestamp::into_inner(ent))
    }

    pub fn add_entry(&mut self, pc: AbsPc, target: AbsPc) {
        self.cache.insert(pc, WithLruTimestamp::new(target));
    }
}

#[derive(Debug, Clone, Default)]
pub struct BranchPredictor {
    btb: BranchTargetBuffer,
    last_taken_map: HashMap<AbsPc, bool>,
}

impl BranchPredictor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn predict_direct(&self, pc: AbsPc, target: AbsPc) -> bool {
        // Simple one-bit history table with static BT, FNT fallback
        self.last_taken_map.get(&pc).copied().unwrap_or(target < pc)
    }

    pub fn update_predict_direct(&mut self, pc: AbsPc, taken: bool) {
        self.last_taken_map.insert(pc, taken);
    }

    pub fn update_predict_indirect(&mut self, pc: AbsPc, target: AbsPc) {
        self.btb.add_entry(pc, target);
    }

    pub fn predict_indirect(&mut self, pc: AbsPc) -> Option<AbsPc> {
        self.btb.get(pc)
    }
}
