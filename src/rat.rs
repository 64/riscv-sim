use crate::inst::ArchReg;
use std::{collections::HashMap, mem};

// #[derive(Debug, Clone)]
// pub struct RegisterAlias(u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RatEntry {
    Valid(u32),
    Invalid(u32), // RS ID.
}

#[derive(Debug, Clone, Default)]
pub struct RegisterAliasTable {
    map: HashMap<ArchReg, RatEntry>,
}

impl RegisterAliasTable {
    pub fn new(initial_regs: HashMap<ArchReg, u32>) -> Self {
        Self {
            map: initial_regs
                .into_iter()
                .map(|(k, v)| (k, RatEntry::Valid(v)))
                .collect(),
        }
    }

    pub fn get(&self, reg: ArchReg) -> RatEntry {
        *self.map.get(&reg).unwrap_or(&RatEntry::Valid(0))
    }

    pub fn rename(&mut self, reg: ArchReg, tag: u32) {
        debug_assert_eq!(
            mem::discriminant(self.map.get(&reg).unwrap_or(&RatEntry::Valid(0))),
            mem::discriminant(&RatEntry::Valid(0))
        );

        self.map.insert(reg, RatEntry::Invalid(tag));
    }

    pub fn set_value(&mut self, reg: ArchReg, val: u32) {
        self.map.insert(reg, RatEntry::Valid(val));
    }
}
