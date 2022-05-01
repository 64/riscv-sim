use std::{fmt, time::Instant};

use crate::{
    execution_unit::{EuType, ExecutionUnit},
    mem::MainMemory,
    program::Program,
    regs::RegSet,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CpuState {
    Running,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub start: Instant,
    pub cycles_taken: u64,
    pub insts_retired: u64,
    pub mispredicts: u64,
    pub rob_stalls: u64,
    pub reservation_station_stalls: u64,
    pub lsq_stalls: u64,
    pub phys_reg_stalls: u64,
    pub l1_miss: u64,
    pub l2_miss: u64,
    pub l3_miss: u64,
    pub eu_util: Vec<(EuType, f32)>,
}

#[derive(Clone)]
pub struct ExecResult {
    pub mem: MainMemory,
    pub regs: RegSet,
    pub stats: Stats,
}

pub trait Cpu {
    fn new(prog: Program, in_regs: RegSet, in_mem: MainMemory) -> Self;

    fn exec_all(self) -> ExecResult;
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            cycles_taken: 0,
            insts_retired: 0,
            mispredicts: 0,
            rob_stalls: 0,
            reservation_station_stalls: 0,
            lsq_stalls: 0,
            phys_reg_stalls: 0,
            l1_miss: 0,
            l2_miss: 0,
            l3_miss: 0,
            eu_util: Vec::new(),
            start: Instant::now(),
        }
    }
}

impl Stats {
    pub fn calculate_util(mut self, eus: &[ExecutionUnit]) -> Self {
        for eu in eus {
            self.eu_util
                .push((eu.eu_type, eu.utilisation as f32 / self.cycles_taken as f32));
        }

        self
    }
}

impl fmt::Debug for ExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult")
            .field("regs", &self.regs)
            .field("stats", &self.stats)
            .finish()
    }
}

impl fmt::Display for ExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "    EXECUTION COMPLETED")?;
        writeln!(f, "  =======================")?;
        if self.stats.phys_reg_stalls != 0 {
            writeln!(
                f,
                "    Phys register stalls: {}",
                self.stats.phys_reg_stalls
            )?;
        }
        if self.stats.lsq_stalls != 0 {
            writeln!(f, " Load/store queue stalls: {}", self.stats.lsq_stalls)?;
        }
        if self.stats.reservation_station_stalls != 0 {
            writeln!(
                f,
                "              R/S stalls: {}",
                self.stats.reservation_station_stalls
            )?;
        }
        if self.stats.rob_stalls != 0 {
            writeln!(f, "   Reorder buffer stalls: {}", self.stats.rob_stalls)?;
        }
        if self.stats.mispredicts != 0 {
            writeln!(f, "      Branch mispredicts: {}", self.stats.mispredicts)?;
        }
        if self.stats.l1_miss != 0 {
            writeln!(f, "         L1 cache misses: {}", self.stats.l1_miss)?;
        }
        if self.stats.l2_miss != 0 {
            writeln!(f, "         L2 cache misses: {}", self.stats.l2_miss)?;
        }
        if self.stats.l3_miss != 0 {
            writeln!(f, "         L3 cache misses: {}", self.stats.l3_miss)?;
        }

        writeln!(f, "    Instructions retired: {}", self.stats.insts_retired)?;
        writeln!(f, "            Cycles taken: {}", self.stats.cycles_taken)?;
        writeln!(
            f,
            "  Instructions per clock: {:.2}",
            self.stats.insts_retired as f32 / self.stats.cycles_taken as f32
        )?;
        writeln!(
            f,
            "  Simulator time elapsed: {:.2}s",
            self.stats.start.elapsed().as_secs_f32()
        )?;

        if self.stats.eu_util.len() > 0 {
            writeln!(f, "          EU utilisation:")?;
            for (eu_type, util) in &self.stats.eu_util {
                if *eu_type != EuType::Special {
                    writeln!(
                        f,
                        "{:>23} = {:>2.0}%",
                        format!("{:?}", eu_type),
                        util * 100.0
                    )?;
                }
            }
        }

        Ok(())
    }
}
