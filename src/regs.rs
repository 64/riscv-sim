use strum::IntoEnumIterator;

use crate::{
    inst::{ArchReg, BothReg, Inst, MemRef, PhysReg, RenamedInst, Tag, ValueOrReg},
    rob::ReorderBuffer,
    util::Addr,
};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct RegSet {
    regs: HashMap<ArchReg, u32>,
}

type RAT = HashMap<ArchReg, PhysReg>;

// See https://docs.boom-core.org/en/latest/sections/rename-stage.html#the-free-list
#[derive(Debug, Clone, Default)]
pub struct BranchInfo {
    rat_cp: RAT,
    alloc_list: Vec<PhysReg>,
    prrt: VecDeque<PhysReg>,
    taken: bool,
    taken_pc: u32,
    not_taken_pc: u32,
}

#[derive(Debug, Clone)]
pub struct RegFile {
    rat: RAT,
    phys_rf: Vec<PrfEntry>,
    prrt: VecDeque<PhysReg>,
    branch_info: HashMap<Tag, BranchInfo>,
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
            branch_info: Default::default(),
        };

        for reg in ArchReg::iter() {
            if reg == ArchReg::Zero {
                continue;
            }

            let val = initial_regs.get(&reg).unwrap_or(&0);
            let slot = rf.allocate_phys_internal().unwrap();
            rf.set_phys_active(slot, *val);
            rf.set_alias(reg, slot);
        }

        rf
    }

    pub fn was_predicted_taken(&self, branch: Tag) -> bool {
        self.branch_info.get(&branch).expect("no branch").taken
    }

    pub fn begin_predict(&mut self, branch: Tag, taken: bool, taken_pc: u32, not_taken_pc: u32) {
        // Create snapshot of the RAT and note whether the branch was taken.
        self.branch_info.insert(
            branch,
            BranchInfo {
                rat_cp: self.rat.clone(),
                alloc_list: Vec::new(),
                prrt: self.prrt.clone(),
                taken,
                taken_pc,
                not_taken_pc,
            },
        );
    }

    pub fn end_predict(&mut self, branch: Tag, taken: bool, predicted: bool) -> Option<u32> {
        let branch_info = self.branch_info.remove(&branch).unwrap();

        if taken != predicted {
            self.rat = branch_info.rat_cp;
            self.prrt = branch_info.prrt;

            for phys_reg in branch_info.alloc_list {
                self.phys_rf[usize::from(phys_reg)] = PrfEntry::Free;
            }

            if taken {
                Some(branch_info.taken_pc)
            } else {
                Some(branch_info.not_taken_pc)
            }
        } else {
            None
        }
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        // TODO: Is this needed?
        self.branch_info.retain(|&t, _| t <= tag);
    }

    pub fn is_speculating(&self, tag: Tag) -> bool {
        self.branch_info.contains_key(&tag)
    }

    fn allocate_phys_internal(&mut self) -> Option<PhysReg> {
        let slot = self.phys_rf.iter().position(|&r| r == PrfEntry::Free)?;
        self.phys_rf[slot] = PrfEntry::Reserved;

        Some(PhysReg::from(slot))
    }

    pub fn allocate_phys(&mut self) -> Option<PhysReg> {
        self.allocate_phys_internal().map(|slot| {
            for branch in self.branch_info.values_mut() {
                branch.alloc_list.push(slot);
            }

            slot
        })
    }

    pub fn release_phys(&mut self) {
        let slot = self
            .prrt
            .pop_front()
            .expect("released PRRT entry when none was allocated");
        self.phys_rf[usize::from(slot)] = PrfEntry::Free;
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
            .get(usize::from(phys_reg))
            .expect("phys reg out of bounds")
    }

    pub fn set_phys_active(&mut self, phys_reg: PhysReg, val: u32) {
        self.phys_rf[usize::from(phys_reg)] = PrfEntry::Active(val);
    }

    pub fn set_alias(&mut self, arch_reg: ArchReg, phys_reg: PhysReg) {
        if arch_reg == ArchReg::Zero {
            todo!();
            // return;
        }

        self.rat.insert(arch_reg, phys_reg);
    }

    pub fn perform_rename(&mut self, tag: Tag, inst: Inst, rob: &ReorderBuffer) -> Option<RenamedInst> {
        let mut should_stall = false;

        let renamed_inst = inst.clone().map_src_regs(|src_reg| match src_reg {
            ArchReg::Zero => ValueOrReg::Value(0),
            src_reg => ValueOrReg::Reg(self.get_alias(src_reg)),
        });
        let renamed_inst = renamed_inst.map_dst_regs(|dst_reg| {
            if dst_reg == ArchReg::Zero {
                BothReg {
                    arch: ArchReg::Zero,
                    phys: PhysReg::none(),
                }
            } else if let Some(slot) = self.allocate_phys() {
                // Prepare the old PhysReg for reclaim.
                let old_phys = self.get_alias(dst_reg);
                self.prrt.push_back(old_phys);

                self.set_alias(dst_reg, slot);
                BothReg {
                    arch: dst_reg,
                    phys: slot,
                }
            } else {
                should_stall = true; // PRF full.

                BothReg {
                    arch: dst_reg,
                    phys: PhysReg::default(),
                }
            }
        });

        if should_stall {
            None
        } else {
            Some(renamed_inst)
        }
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
