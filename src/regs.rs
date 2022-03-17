use strum::IntoEnumIterator;

use crate::{
    inst::{ArchReg, MemRef, PhysReg, Tag},
    util::Addr,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegSet {
    regs: HashMap<ArchReg, u32>,
}

#[derive(Debug, Clone)]
pub struct RegFile {
    front_rf: ArchRegFile,
    back_rf: ArchRegFile,
    phys_rf: PhysRegFile,
}

// https://ece.uwaterloo.ca/~maagaard/ece720-t4/lec-05.pdf
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrfEntry {
    Free,
    Reserved,
    Active(u32),
    Unused,
}

#[derive(Debug, Clone)]
struct PhysRegFile {
    map: Vec<PrfEntry>,
}

#[derive(Debug, Clone, Default)]
struct ArchRegFile {
    map: HashMap<ArchReg, PhysReg>,
}

impl PhysRegFile {
    pub fn new(capacity: usize) -> Self {
        assert!(PhysReg::try_from(capacity).is_ok());
        Self {
            map: vec![PrfEntry::Free; capacity],
        }
    }
}

impl RegFile {
    pub fn new(initial_regs: HashMap<ArchReg, u32>, prf_capacity: usize) -> Self {
        assert!(initial_regs.len() <= prf_capacity);

        let mut rf = Self {
            phys_rf: PhysRegFile::new(prf_capacity),
            back_rf: ArchRegFile::default(),
            front_rf: ArchRegFile::default(),
        };

        for reg in ArchReg::iter() {
            if reg == ArchReg::Zero {
                continue;
            }

            let val = initial_regs.get(&reg).unwrap_or(&0);
            let slot = rf.allocate_phys().unwrap();
            rf.set_phys_active(slot, *val);
            rf.set_back(reg, slot);
            rf.set_front(reg, slot);
        }

        rf
    }

    pub fn allocate_phys(&mut self) -> Option<PhysReg> {
        let slot = self.phys_rf.map.iter().position(|&r| r == PrfEntry::Free)?;
        self.phys_rf.map[slot] = PrfEntry::Reserved;
        Some(PhysReg::try_from(slot).unwrap())
    }

    pub fn get_front(&self, arch_reg: ArchReg) -> PhysReg {
        if arch_reg == ArchReg::Zero {
            return PhysReg::try_from(0).unwrap();
        }

        self.front_rf.map.get(&arch_reg).copied().unwrap()
    }

    pub fn get_phys(&self, phys_reg: PhysReg) -> PrfEntry {
        *self
            .phys_rf
            .map
            .get(usize::try_from(phys_reg).unwrap())
            .expect("phys reg out of bounds")
    }

    pub fn set_phys_active(&mut self, phys_reg: PhysReg, val: u32) {
        self.phys_rf.map[usize::try_from(phys_reg).unwrap()] = PrfEntry::Active(val);
    }

    pub fn set_front(&mut self, arch_reg: ArchReg, phys_reg: PhysReg) {
        if arch_reg == ArchReg::Zero {
            todo!();
            // return;
        }

        self.front_rf.map.insert(arch_reg, phys_reg);
    }

    pub fn set_back(&mut self, arch_reg: ArchReg, phys_reg: PhysReg) {
        if arch_reg == ArchReg::Zero {
            return;
        }

        self.back_rf.map.insert(arch_reg, phys_reg);
    }
}

impl RegSet {
    pub fn new(regs: HashMap<ArchReg, u32>) -> Self {
        Self { regs }
    }

    pub fn get(&self, reg: ArchReg) -> u32 {
        if reg == ArchReg::Zero {
            0
        } else {
            *self.regs.get(&reg).unwrap_or(&0)
        }
    }

    pub fn set(&mut self, reg: ArchReg, value: u32) {
        if reg != ArchReg::Zero {
            self.regs.insert(reg, value);
        }
    }

    pub fn ref_to_addr(&self, mr: MemRef) -> Addr {
        Addr(self.get(mr.base).wrapping_add(mr.offset.0))
    }
}
