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
pub struct Start {
    time: Instant,
}

impl Default for Start {
    fn default() -> Start {
        Start {
            time: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub start: Start,
    pub cycles_taken: u64,
    pub insts_retired: u64,
    pub direct_mispredicts: u64,
    pub indirect_mispredicts: u64,
    pub rob_stalls: u64,
    pub reservation_station_stalls: u64,
    pub lsq_stalls: u64,
    pub phys_reg_stalls: u64,
    pub fetch_stalls: u64,
    pub l1_miss: u64,
    pub l2_miss: u64,
    pub l3_miss: u64,
    pub l1_hits: u64,
    pub l2_hits: u64,
    pub l3_hits: u64,
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
        if self.stats.fetch_stalls != 0 {
            writeln!(f, "            Fetch stalls: {}", self.stats.fetch_stalls)?;
        }
        if self.stats.rob_stalls != 0 {
            writeln!(f, "   Reorder buffer stalls: {}", self.stats.rob_stalls)?;
        }
        if self.stats.direct_mispredicts != 0 {
            writeln!(
                f,
                "      Direct mispredicts: {}",
                self.stats.direct_mispredicts
            )?;
        }
        if self.stats.indirect_mispredicts != 0 {
            writeln!(
                f,
                "    Indirect mispredicts: {}",
                self.stats.indirect_mispredicts
            )?;
        }
        if self.stats.l1_hits != 0 {
            writeln!(f, "           L1 cache hits: {}", self.stats.l1_hits)?;
        }
        if self.stats.l1_miss != 0 {
            writeln!(f, "         L1 cache misses: {}", self.stats.l1_miss)?;
        }
        if self.stats.l2_hits != 0 {
            writeln!(f, "           L2 cache hits: {}", self.stats.l2_hits)?;
        }
        if self.stats.l2_miss != 0 {
            writeln!(f, "         L2 cache misses: {}", self.stats.l2_miss)?;
        }
        if self.stats.l3_hits != 0 {
            writeln!(f, "           L3 cache hits: {}", self.stats.l3_hits)?;
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
            self.stats.start.time.elapsed().as_secs_f32()
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
