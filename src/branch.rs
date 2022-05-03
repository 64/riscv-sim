#![allow(dead_code, unused)]

use hashbrown::HashMap;

use crate::inst::{AbsPc, ArchReg, Imm, Inst, INST_SIZE};

#[derive(Debug, Clone, Default)]
pub struct BranchPredictor {
    btb: HashMap<AbsPc, AbsPc>,
    ras: Vec<AbsPc>,
    last_taken_map: HashMap<AbsPc, i32>,
}

impl BranchPredictor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn predict_direct(&self, pc: AbsPc, target: AbsPc) -> bool {
        // Simple one-bit history table with static BT, FNT fallback
        self.last_taken_map
            .get(&pc)
            .copied()
            .map(|state| state >= 0)
            .unwrap_or(target < pc)
    }

    pub fn update_predict_direct(&mut self, pc: AbsPc, taken: bool) {
        let state = match self.last_taken_map.get(&pc) {
            Some(state) => {
                if taken {
                    state + 1
                } else {
                    state - 1
                }
            }
            None => i32::from(taken) - 1,
        };

        const NBITS: u32 = 2;
        let max_state = 2_i32.pow(NBITS - 1);
        let state = state.max(-max_state).min(max_state - 1);

        self.last_taken_map.insert(pc, state);
    }

    pub fn update_predict_indirect(&mut self, pc: AbsPc, target: AbsPc) {
        self.btb.insert(pc, target);
    }

    pub fn predict_indirect(&mut self, inst: &Inst, pc: AbsPc) -> Option<AbsPc> {
        if matches!(inst, Inst::JumpAndLink(ArchReg::RA, _)) {
            // Call
            self.ras.push(pc + INST_SIZE);
            // println!("Call with RA = {:?}", pc + INST_SIZE);
            self.btb.get(&pc).copied()
        } else if inst == &Inst::JumpAndLinkRegister(ArchReg::Zero, ArchReg::RA, Imm(0)) {
            // Ret
            self.ras.pop()
            // println!("Return from {:?} to {:?}", pc, val);
            // self.btb.get(&pc).copied()
        } else {
            self.btb.get(&pc).copied()
        }
    }
}
