use crate::{
    branch::BranchPredictor,
    inst::{
        AbsPc, ArchReg, BothReg, Inst, MemRef, PhysReg, RenamedInst, Tag, ValueOrReg, INST_SIZE,
    },
    mem,
    util::Addr,
};
use hashbrown::HashMap;
use std::{collections::VecDeque, fmt};
use strum::IntoEnumIterator;

#[derive(Clone, Default)]
pub struct RegSet {
    regs: HashMap<ArchReg, u32>,
}

type AliasTable = HashMap<ArchReg, PhysReg>;

#[derive(Debug, Clone)]
enum BranchType {
    Direct {
        taken: bool,
        taken_pc: AbsPc,
        not_taken_pc: AbsPc,
    },
    Indirect {
        predicted_pc: Option<AbsPc>,
        inst_pc: AbsPc,
    },
}

// See https://docs.boom-core.org/en/latest/sections/rename-stage.html#the-free-list
#[derive(Debug, Clone)]
pub struct BranchInfo {
    // Options because we snapshot during rename (so that everything is in-order)
    rat_cp: Option<AliasTable>,
    alloc_list: Option<Vec<PhysReg>>,
    info: BranchType,
}

#[derive(Clone)]
pub struct PhysFile(Vec<PrfEntry>);

#[derive(Debug, Clone)]
pub struct RegFile {
    rat: AliasTable,
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
    pub fn new(initial_regs: RegSet, prf_capacity: usize) -> Self {
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

            let slot = rf.allocate_phys_internal().unwrap();
            rf.set_phys_active(slot, initial_regs.get(reg));
            rf.set_alias(reg, slot);
        }

        rf
    }

    #[allow(dead_code)]
    pub fn is_prrt_empty(&self) -> bool {
        self.prrt.is_empty()
    }

    pub fn get_reg_set(self) -> RegSet {
        let map: HashMap<ArchReg, u32> = self
            .rat
            .iter()
            .map(|(&k, &v)| match self.phys_rf.0[usize::from(v)] {
                PrfEntry::Active(v) => (k, v),
                // _ => unreachable!(),
                _ => (k, 0),
            })
            .collect();
        RegSet::from(map)
    }

    pub fn was_predicted_taken(&self, branch: Tag) -> bool {
        match self
            .branch_info
            .get(&branch)
            .expect("no branch info for direct branch")
            .info
        {
            BranchType::Direct { taken, .. } => taken,
            _ => unreachable!(),
        }
    }

    pub fn begin_predict_direct(
        &mut self,
        branch: Tag,
        taken: bool,
        taken_pc: AbsPc,
        not_taken_pc: AbsPc,
    ) {
        self.branch_info.insert(
            branch,
            BranchInfo {
                rat_cp: None,
                alloc_list: None,
                info: BranchType::Direct {
                    taken,
                    taken_pc,
                    not_taken_pc,
                },
            },
        );
    }

    pub fn begin_predict_indirect(
        &mut self,
        branch: Tag,
        predicted_pc: Option<AbsPc>,
        inst_pc: AbsPc,
    ) {
        self.branch_info.insert(
            branch,
            BranchInfo {
                rat_cp: None,
                alloc_list: None,
                info: BranchType::Indirect {
                    predicted_pc,
                    inst_pc,
                },
            },
        );
    }

    pub fn end_predict_indirect(
        &mut self,
        branch: Tag,
        actual_pc: AbsPc,
        predicted_pc: Option<AbsPc>,
        branch_predictor: &mut BranchPredictor,
    ) {
        let branch_info = self.branch_info.remove(&branch).unwrap();

        match branch_info.info {
            BranchType::Indirect { inst_pc, .. } => {
                branch_predictor.update_predict_indirect(inst_pc, actual_pc);
            }
            _ => unreachable!(),
        }

        if predicted_pc
            .map(|predicted_pc| actual_pc != predicted_pc)
            .unwrap_or(false)
        {
            self.mispredict(branch, &branch_info);
        }
    }

    pub fn end_predict_direct(
        &mut self,
        branch: Tag,
        taken: bool,
        predicted: bool,
        branch_predictor: &mut BranchPredictor,
    ) -> Option<AbsPc> {
        let branch_info = self.branch_info.remove(&branch).unwrap();

        match branch_info.info {
            BranchType::Direct { not_taken_pc, .. } => {
                let inst_pc = not_taken_pc - INST_SIZE;
                branch_predictor.update_predict_direct(inst_pc, taken);
            }
            _ => unreachable!(),
        }

        if taken != predicted {
            self.mispredict(branch, &branch_info);

            match branch_info.info {
                BranchType::Direct {
                    taken_pc,
                    not_taken_pc,
                    ..
                } => {
                    if taken {
                        Some(taken_pc)
                    } else {
                        Some(not_taken_pc)
                    }
                }
                _ => unreachable!(),
            }
        } else {
            None
        }
    }

    fn mispredict(&mut self, tag: Tag, info: &BranchInfo) {
        let alloc_list = info.alloc_list.as_ref().unwrap();
        self.rat = info.rat_cp.as_ref().unwrap().clone();

        let num_removed = alloc_list.len();

        for &phys_reg in alloc_list {
            self.phys_rf.0[usize::from(phys_reg)] = PrfEntry::Free;
            self.prrt.pop_back().unwrap();
        }

        // self.branch_info.iter().for_each(|(&t, _)| debug_assert!(t >= tag));
        self.branch_info.retain(|&t, _| t <= tag);

        // Is this needed?
        for bi in self.branch_info.values_mut() {
            for _ in 0..num_removed {
                bi.alloc_list.as_mut().map(|al| al.pop().unwrap());
            }
        }
    }

    pub fn kill_tags_after(&mut self, tag: Tag) {
        // // TODO: Is this needed?
        self.branch_info
            .iter()
            .for_each(|(&t, _)| debug_assert!(t <= tag));
    }

    fn allocate_phys_internal(&mut self) -> Option<PhysReg> {
        let slot = self.phys_rf.0.iter().position(|&r| r == PrfEntry::Free)?;
        self.phys_rf.0[slot] = PrfEntry::Reserved;

        Some(PhysReg::from(slot))
    }

    pub fn allocate_phys(&mut self, _tag: Tag) -> Option<PhysReg> {
        self.allocate_phys_internal().map(|slot| {
            for branch in self.branch_info.values_mut() {
                if let Some(al) = branch.alloc_list.as_mut() {
                    al.push(slot)
                }
            }

            slot
        })
    }

    pub fn release_phys(&mut self, _tag: Tag) {
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

    pub fn perform_rename(&mut self, tag: Tag, inst: Inst) -> Option<RenamedInst> {
        if let Some(bi) = self.branch_info.get_mut(&tag) {
            bi.rat_cp = Some(self.rat.clone());
            bi.alloc_list = Some(Vec::new());
        }

        // Have to do this in two separate steps to prevent borrowing issues.
        let renamed_inst = inst.map_src_regs(|src_reg| match src_reg {
            ArchReg::Zero => ValueOrReg::Value(0),
            src_reg => ValueOrReg::Reg(self.get_alias(src_reg)),
        });

        renamed_inst.try_map(
            Some,
            |dst_reg| {
                if dst_reg == ArchReg::Zero {
                    Some(BothReg {
                        arch: ArchReg::Zero,
                        phys: PhysReg::none(),
                    })
                } else if let Some(slot) = self.allocate_phys(tag) {
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
            Some,
        )
    }

    pub fn predicted_addr(&self, tag: Tag) -> (Option<AbsPc>, AbsPc) {
        match self.branch_info.get(&tag).expect("no branch info").info {
            BranchType::Indirect {
                predicted_pc,
                inst_pc,
            } => (predicted_pc, inst_pc),
            _ => unreachable!(),
        }
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
        fmt.debug_map()
            .entries(
                self.0
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| **v != PrfEntry::Free),
            )
            .finish()
    }
}

impl From<HashMap<ArchReg, u32>> for RegSet {
    fn from(mut regs: HashMap<ArchReg, u32>) -> Self {
        if regs.get(&ArchReg::SP).is_none() {
            regs.insert(ArchReg::SP, mem::STACK_TOP.try_into().unwrap());
        }
        assert_eq!(regs.get(&ArchReg::Zero), None);
        Self { regs }
    }
}

impl<const SIZE: usize> From<[(ArchReg, u32); SIZE]> for RegSet {
    fn from(regs: [(ArchReg, u32); SIZE]) -> Self {
        RegSet::from(HashMap::from(regs))
    }
}

impl RegSet {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
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

impl fmt::Debug for RegSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.regs.iter().filter(|&(_, v)| *v != 0))
            .finish()
    }
}
