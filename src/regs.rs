use strum::IntoEnumIterator;

use crate::{
    inst::{ArchReg, BothReg, Inst, MemRef, PhysReg, RenamedInst, Tag, ValueOrReg},
    util::Addr,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
};

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
    taken: bool,
    taken_pc: u32,
    not_taken_pc: u32,
}

#[derive(Clone)]
pub struct PhysFile(Vec<PrfEntry>);

#[derive(Debug, Clone)]
pub struct RegFile {
    rat: RAT,
    phys_rf: PhysFile,
    prrt: VecDeque<PhysReg>,
    branch_info: HashMap<Tag, BranchInfo>,
}

// https://ece.uwaterloo.ca/~maagaard/ece720-t4/lec-05.pdf
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PrfEntry {
    Free,
    Reserved,
    Active(u32),
}

impl RegFile {
    pub fn new(initial_regs: HashMap<ArchReg, u32>, prf_capacity: usize) -> Self {
        assert!(
            ArchReg::iter().count() <= prf_capacity,
            "prf not large enough"
        );
        assert!(PhysReg::try_from(prf_capacity).is_ok());

        let mut rf = Self {
            rat: Default::default(),
            phys_rf: PhysFile(vec![PrfEntry::Free; prf_capacity]),
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

    pub fn get_reg_set(self) -> RegSet {
        let map: HashMap<ArchReg, u32> = self
            .rat
            .iter()
            .map(|(&k, &v)| match self.phys_rf.0[usize::from(v)] {
                PrfEntry::Active(v) => (k, v),
                _ => unreachable!(),
            })
            .collect();
        RegSet::new(map)
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

            for phys_reg in branch_info.alloc_list {
                self.phys_rf.0[usize::from(phys_reg)] = PrfEntry::Free;
                self.prrt.pop_back();
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

    fn allocate_phys_internal(&mut self) -> Option<PhysReg> {
        let slot = self.phys_rf.0.iter().position(|&r| r == PrfEntry::Free)?;
        self.phys_rf.0[slot] = PrfEntry::Reserved;

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
        let slot = &mut self.phys_rf.0[usize::from(slot)];
        assert!(*slot != PrfEntry::Free && *slot != PrfEntry::Reserved);
        *slot = PrfEntry::Free;
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
            .0
            .get(usize::from(phys_reg))
            .expect("phys reg out of bounds")
    }

    pub fn set_phys_active(&mut self, phys_reg: PhysReg, val: u32) {
        self.phys_rf.0[usize::from(phys_reg)] = PrfEntry::Active(val);
    }

    pub fn set_alias(&mut self, arch_reg: ArchReg, phys_reg: PhysReg) {
        if arch_reg == ArchReg::Zero {
            unreachable!();
        }

        self.rat.insert(arch_reg, phys_reg);
    }

    pub fn perform_rename(&mut self, inst: Inst) -> Option<RenamedInst> {
        // Have to do this in two separate steps to prevent borrowing issues.
        let renamed_inst = inst.clone().map_src_regs(|src_reg| match src_reg {
            ArchReg::Zero => ValueOrReg::Value(0),
            src_reg => ValueOrReg::Reg(self.get_alias(src_reg)),
        });
        let renamed_inst = renamed_inst.try_map(
            |src_reg| Some(src_reg),
            |dst_reg| {
                if dst_reg == ArchReg::Zero {
                    Some(BothReg {
                        arch: ArchReg::Zero,
                        phys: PhysReg::none(),
                    })
                } else if let Some(slot) = self.allocate_phys() {
                    // Prepare the old PhysReg for reclaim.
                    let old_phys = self.get_alias(dst_reg);
                    self.prrt.push_back(old_phys);
                    self.set_alias(dst_reg, slot);
                    Some(BothReg {
                        arch: dst_reg,
                        phys: slot,
                    })
                } else {
                    None
                }
            },
            |jump| Some(jump),
        );

        renamed_inst
    }
}

impl fmt::Debug for PrfEntry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrfEntry::Active(x) => write!(fmt, "Active({x})"),
            PrfEntry::Free => write!(fmt, "Free"),
            PrfEntry::Reserved => write!(fmt, "Reserved"),
        }
    }
}

impl fmt::Debug for PhysFile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_map().entries(self.0.iter().enumerate()).finish()
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
