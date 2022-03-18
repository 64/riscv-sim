use strum::IntoEnumIterator;

use crate::{
    inst::{ArchReg, MemRef, PhysReg},
    util::Addr,
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct RegSet {
    regs: HashMap<ArchReg, u32>,
}

#[derive(Debug, Clone)]
pub struct RegFile {
    rat: HashMap<ArchReg, PhysReg>,
    phys_rf: Vec<PrfEntry>,
    prrt: VecDeque<PhysReg>, // Post-retirement reclaim table
}

// https://ece.uwaterloo.ca/~maagaard/ece720-t4/lec-05.pdf
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrfEntry {
    Free,
    Reserved,
    Active(u32),
}

impl RegFile {
    pub fn new(initial_regs: HashMap<ArchReg, u32>, prf_capacity: usize) -> Self {
        assert!(initial_regs.len() <= prf_capacity);
        assert!(PhysReg::try_from(prf_capacity).is_ok());

        let mut rf = Self {
            rat: Default::default(),
            phys_rf: vec![PrfEntry::Free; prf_capacity],
            prrt: Default::default(),
        };

        for reg in ArchReg::iter() {
            if reg == ArchReg::Zero {
                continue;
            }

            let val = initial_regs.get(&reg).unwrap_or(&0);
            let slot = rf.allocate_phys().unwrap();
            rf.set_phys_active(slot, *val);
            rf.set_alias(reg, slot);
        }

        rf
    }

    pub fn allocate_phys(&mut self) -> Option<PhysReg> {
        let slot = self.phys_rf.iter().position(|&r| r == PrfEntry::Free)?;
        self.phys_rf[slot] = PrfEntry::Reserved;

        // Allocate an entry in the PRRT.
        self.prrt.push_back(slot.try_into().unwrap());

        Some(PhysReg::try_from(slot).unwrap())
    }

    pub fn release_phys(&mut self) {
        let slot = self
            .prrt
            .pop_front()
            .expect("released PRRT entry when none was allocated");
        self.phys_rf[usize::try_from(slot).unwrap()] = PrfEntry::Free;
    }

    pub fn get_alias(&self, arch_reg: ArchReg) -> PhysReg {
        if arch_reg == ArchReg::Zero {
            return PhysReg::try_from(0).unwrap();
        }

        self.rat.get(&arch_reg).copied().unwrap()
    }

    pub fn get_phys(&self, phys_reg: PhysReg) -> PrfEntry {
        *self
            .phys_rf
            .get(usize::try_from(phys_reg).unwrap())
            .expect("phys reg out of bounds")
    }

    pub fn set_phys_active(&mut self, phys_reg: PhysReg, val: u32) {
        self.phys_rf[usize::try_from(phys_reg).unwrap()] = PrfEntry::Active(val);
    }

    pub fn set_alias(&mut self, arch_reg: ArchReg, phys_reg: PhysReg) {
        if arch_reg == ArchReg::Zero {
            todo!();
            // return;
        }

        self.rat.insert(arch_reg, phys_reg);
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
