use crate::inst::{ArchReg, Tag, ValueOrTag};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RegisterAliasTable {
    map: HashMap<ArchReg, ValueOrTag>,
}

impl RegisterAliasTable {
    pub fn new(initial_regs: HashMap<ArchReg, u32>) -> Self {
        Self {
            map: initial_regs
                .into_iter()
                .map(|(k, v)| (k, ValueOrTag::Valid(v)))
                .collect(),
        }
    }

    pub fn get(&self, reg: ArchReg) -> ValueOrTag {
        if reg == ArchReg::Zero {
            return ValueOrTag::Valid(0);
        }

        self.map.get(&reg).cloned().unwrap_or(ValueOrTag::Valid(0))
    }

    pub fn rename(&mut self, reg: ArchReg, tag: Tag) -> bool {
        if reg == ArchReg::Zero {
            return false;
        }

        // #[cfg(debug_assertions)]
        if let Some(ValueOrTag::Invalid(old_tag)) = self.map.get(&reg) {
            return true;
            // panic!(
            //     "tried to rename register {:?} when it was already renamed: old = {:?}, new = {:?}",
            //     reg, old_tag, tag
            // );
        }

        self.map.insert(reg, ValueOrTag::Invalid(tag));
        false
    }

    pub fn set_value(&mut self, reg: ArchReg, val: u32) {
        if reg == ArchReg::Zero {
            return;
        }

        self.map.insert(reg, ValueOrTag::Valid(val));
    }
}
