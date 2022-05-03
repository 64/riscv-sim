use hashbrown::HashMap;

use crate::{
    branch::BranchPredictor,
    cpu::{Cpu, ExecResult, Stats},
    execution_unit::{EuType, ExecutionUnit},
    inst::{AbsPc, ArchReg, ExecutedInst, Imm, Inst, RenamedInst, Tag, Tagged, INST_SIZE},
    lsq::LoadStoreQueue,
    mem::{MainMemory, MemoryHierarchy},
    program::Program,
    regs::{RegFile, RegSet},
    reservation_station::ReservationStation,
    rob::ReorderBuffer,
};

mod stages {
    use super::*;

    pub mod narrow {
        use super::*;

        #[derive(Debug, Clone)]
        pub struct FetchDecode {
            pub inst: Tagged<Inst>,
            pub next_pc: Option<AbsPc>, // None if we should stall
        }

        #[derive(Debug, Clone)]
        pub struct Rename {
            pub inst: Option<RenamedInst>,
            pub should_stall: bool,
        }

        #[derive(Debug, Clone)]
        pub struct Writeback {
            pub inst: Option<Tagged<ExecutedInst>>,
            pub next_fetch: Option<AbsPc>,
        }
    }

    pub mod wide {
        use super::*;

        #[derive(Debug, Clone, Default)]
        pub struct FetchDecode {
            pub insts: Vec<Tagged<Inst>>,
            pub next_pcs: Vec<AbsPc>,
            pub stalled: bool,
        }

        #[derive(Debug, Clone, Default)]
        pub struct Rename {
            pub insts: Vec<RenamedInst>,
            pub next_fetch_decode: Option<FetchDecode>,
        }

        #[derive(Debug, Clone, Default)]
        pub struct Issue {
            pub next_fetch: Option<AbsPc>,
        }

        #[derive(Debug, Clone, Default)]
        pub struct Writeback {
            pub insts: Vec<Tagged<ExecutedInst>>,
            pub next_fetch: Option<AbsPc>,
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct Commit {
        pub should_halt: bool,
    }
}

#[derive(Debug, Clone, Default)]
#[allow(unused)]
pub struct Pipeline {
    fetch_decode: stages::wide::FetchDecode,
    rename: stages::wide::Rename,
    writeback: stages::wide::Writeback,
    commit: stages::Commit,
}

#[derive(Debug, Clone)]
pub struct OutOfOrder {
    mem: MemoryHierarchy,
    prog: Program,
    execution_units: Vec<ExecutionUnit>,
    pc_map: HashMap<Tag, AbsPc>,
    reservation_station: ReservationStation,
    lsq: LoadStoreQueue,
    rob: ReorderBuffer,
    branch_predictor: BranchPredictor,
    reg_file: RegFile,
    stats: Stats,
}

const PIPE_WIDTH: u64 = 4;
const MACRO_OP_FUSE: bool = true;

impl Cpu for OutOfOrder {
    fn new(prog: Program, regs: RegSet, mem: MainMemory) -> Self {
        Self {
            mem: MemoryHierarchy::new(mem),
            prog,
            execution_units: vec![
                ExecutionUnit::new(EuType::Branch),
                ExecutionUnit::new(EuType::LoadStore),
                ExecutionUnit::new(EuType::Alu),
                ExecutionUnit::new(EuType::Alu),
                ExecutionUnit::new(EuType::Special),
            ],
            rob: ReorderBuffer::new(250),
            lsq: LoadStoreQueue::new(70, 70),
            pc_map: HashMap::new(),
            reservation_station: ReservationStation::new(100),
            reg_file: RegFile::new(regs, 200),
            branch_predictor: BranchPredictor::new(),
            stats: Stats::default(),
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut pipe = Pipeline::default();
        pipe.fetch_decode.next_pcs.push(AbsPc(0));

        loop {
            self.mem.tick();

            let commit = self.stage_commit(&pipe);
            let writeback = self.stage_writeback(&pipe);

            if commit.should_halt {
                // assert!(self.reg_file.is_prrt_empty());

                return ExecResult {
                    regs: self.reg_file.get_reg_set(),
                    mem: self.mem.main,
                    stats: self.stats.calculate_util(&self.execution_units),
                };
            }

            if let Some(next_pc) = writeback.next_fetch {
                // let issue = self.stage_issue(&pipe);
                // self.stage_execute(&pipe);

                pipe = Pipeline {
                    fetch_decode: stages::wide::FetchDecode {
                        insts: Vec::new(),
                        next_pcs: vec![next_pc],
                        stalled: false,
                    },
                    rename: stages::wide::Rename::default(),
                    writeback,
                    commit,
                };
            } else {
                let issue = self.stage_issue(&pipe);

                if let Some(next_pc) = issue.next_fetch {
                    // println!("JUMPING TO {:?}", next_pc);
                    pipe = Pipeline {
                        fetch_decode: stages::wide::FetchDecode {
                            insts: Vec::new(),
                            next_pcs: vec![next_pc],
                            stalled: false,
                        },
                        rename: stages::wide::Rename::default(),
                        writeback,
                        commit,
                    };
                } else {
                    self.stage_execute(&pipe);
                    let rename = self.stage_rename(&pipe);

                    if let Some(nxt) = &rename.next_fetch_decode {
                        pipe = Pipeline {
                            fetch_decode: nxt.clone(),
                            rename,
                            writeback,
                            commit,
                        };
                    } else {
                        let fetch_decode = self.stage_fetch_decode(&pipe);

                        pipe = Pipeline {
                            fetch_decode,
                            rename,
                            writeback,
                            commit,
                        };
                    }
                }
            }

            self.stats.cycles_taken += 1;

            #[cfg(debug_assertions)]
            if std::env::var("SINGLE_STEP").is_ok() {
                self.dump(&pipe);
                // std::io::stdin().read_line(&mut String::new()).unwrap();
            }

            debug_assert!(
                self.stats.cycles_taken < 1_000_000,
                "infinite loop detected"
            );
        }
    }
}

impl OutOfOrder {
    #[allow(dead_code, unused)]
    fn dump(&self, pipe: &Pipeline) {
        // dbg!(&self.lsq);
        // dbg!(&self.reg_file);
        // dbg!(&self.reservation_station);
        // dbg!(&self.rob);
        dbg!(&self.execution_units);
        // dbg!(pipe);
        // dbg!(self.stats.cycles_taken);

        for Tagged {
            inst: next_inst,
            tag,
        } in pipe.fetch_decode.insts.iter()
        {
            println!("{:?} @ {:?}", next_inst, tag);
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }

    fn fetch_decode_one(&mut self, pc: AbsPc, tag: Tag) -> stages::narrow::FetchDecode {
        let inst = self.prog.fetch(pc).cloned().unwrap_or(Inst::Halt);
        let next_inst = self
            .prog
            .fetch(pc + INST_SIZE)
            .cloned()
            .unwrap_or(Inst::Halt);

        let fused = if MACRO_OP_FUSE {
            #[allow(unused)]
            match (&inst, &next_inst) {
                (Inst::Add(rd1, rs1, rs2), Inst::LoadByte(rd2, mem_ref))
                    if rd1 == rd2 && *rd2 == mem_ref.base =>
                {
                    todo!()
                    // Some(Inst::IndexedLoadByte(*rd2, *rs1, *rs2, mem_ref.offset))
                }
                (Inst::Add(rd1, rs1, rs2), Inst::LoadByteU(rd2, mem_ref))
                    if rd1 == rd2 && *rd2 == mem_ref.base =>
                {
                    Some(Inst::IndexedLoadByteU(*rd2, *rs1, *rs2, mem_ref.offset))
                }
                (Inst::Add(rd1, rs1, rs2), Inst::LoadHalfWord(rd2, mem_ref))
                    if rd1 == rd2 && *rd2 == mem_ref.base =>
                {
                    todo!()
                    // Some(Inst::IndexedLoadHalfWord(*rd2, *rs1, *rs2, mem_ref.offset))
                }
                // (Inst::Add(rd1, rs1, rs2), Inst::LoadHalfWordU(rd2, mem_ref))
                //     if rd1 == rd2 && *rd2 == mem_ref.base =>
                // {
                //     todo!()
                //     // Some(Inst::IndexedLoadHalfWordU(*rd2, *rs1, *rs2, mem_ref.offset))
                // }
                (Inst::Add(rd1, rs1, rs2), Inst::LoadWord(rd2, mem_ref))
                    if rd1 == rd2 && *rd2 == mem_ref.base =>
                {
                    todo!()
                    // Some(Inst::IndexedLoadWord(*rd2, *rs1, *rs2, mem_ref.offset))
                }
                (Inst::ShiftLeftLogicalImm(rd1, rs1, imm), Inst::Add(rd2, rs2, rs3))
                    if rd1 == rd2 && rd2 == rs2 =>
                {
                    Some(Inst::EffectiveAddress(*rd2, *rs1, *rs3, *imm))
                }
                (Inst::LoadUpperImm(rd1, imm1), Inst::AddImm(rd2, rs1, imm2))
                    if rd1 == rd2 && rd2 == rs1 =>
                {
                    let x = (imm1.0 << 12).wrapping_add(imm2.0);
                    Some(Inst::LoadFullImm(*rd2, Imm(x)))
                }
                _ => None,
            }
        } else {
            None
        };

        if let Some(fused) = fused {
            if fused.is_load() {
                self.pc_map.insert(tag, pc);
            }

            stages::narrow::FetchDecode {
                inst: Tagged { inst: fused, tag },
                next_pc: Some(pc + 2 * INST_SIZE),
            }
        } else {
            // Branch prediction
            let next_pc = match &inst {
                Inst::BranchIfEqual(_, _, tgt)
                | Inst::BranchIfNotEqual(_, _, tgt)
                | Inst::BranchIfLess(_, _, tgt)
                | Inst::BranchIfLessU(_, _, tgt)
                | Inst::BranchIfGreaterEqualU(_, _, tgt)
                | Inst::BranchIfGreaterEqual(_, _, tgt) => {
                    let taken_pc = *tgt;
                    let not_taken_pc = pc + INST_SIZE;
                    let predict_taken = self.branch_predictor.predict_direct(pc, taken_pc);

                    self.reg_file
                        .begin_predict_direct(tag, predict_taken, taken_pc, not_taken_pc);
                    // println!("begin predict {:?} at {:?} ({})", inst, self.stats.insts_retired, predict_taken);

                    if predict_taken {
                        Some(taken_pc)
                    } else {
                        Some(not_taken_pc)
                    }
                }
                Inst::JumpAndLink(_, tgt) => {
                    // println!("jal {:?} at {:?}", inst, self.stats.insts_retired);
                    let _ = self.branch_predictor.predict_indirect(&inst, pc); // Update RAS

                    self.pc_map.insert(tag, pc);
                    Some(*tgt)
                }
                Inst::JumpAndLinkRegister(_, _, _) => {
                    // println!("begin predict indirect {:?} at {:?}", inst, self.stats.insts_retired);
                    let predicted_addr = self.branch_predictor.predict_indirect(&inst, pc);
                    self.reg_file
                        .begin_predict_indirect(tag, predicted_addr, pc);
                    predicted_addr
                }
                _ => {
                    debug_assert!(!inst.is_branch());

                    if inst.is_load() {
                        self.pc_map.insert(tag, pc);
                        self.reg_file.begin_predict_mem(tag, pc);
                    }

                    Some(pc + INST_SIZE)
                }
            };

            stages::narrow::FetchDecode {
                inst: Tagged { inst, tag },
                next_pc,
            }
        }
    }

    fn stage_fetch_decode(&mut self, pipe: &Pipeline) -> stages::wide::FetchDecode {
        let next_pc = match pipe.fetch_decode.next_pcs.last() {
            Some(next_pc) => *next_pc,
            None => {
                // Stalled.
                return stages::wide::FetchDecode {
                    insts: Vec::new(),
                    next_pcs: Vec::new(),
                    stalled: true,
                };
            }
        };

        let mut insts = Vec::new();
        let mut next_pcs = vec![next_pc];
        let mut stalled = false;

        for i in 0..PIPE_WIDTH {
            if self.rob.last_is_halt() {
                break;
            }

            let tag = Tag::from(PIPE_WIDTH * self.stats.cycles_taken + i);

            let res = self.fetch_decode_one(*next_pcs.last().unwrap(), tag);

            insts.push(res.inst);

            if let Some(next_pc) = res.next_pc {
                next_pcs.push(next_pc);
            } else {
                self.stats.fetch_stalls += 1;
                stalled = true;
                break;
            }
        }

        stages::wide::FetchDecode {
            insts,
            next_pcs,
            stalled,
        }
    }

    fn stage_rename(&mut self, pipe: &Pipeline) -> stages::wide::Rename {
        let mut insts = vec![];
        let mut num_renamed = 0;
        let mut should_stall = false;

        for inst in &pipe.fetch_decode.insts {
            let renamed = self.rename_one(inst);

            if renamed.should_stall {
                should_stall = true;
                break;
            }

            if let Some(inst) = renamed.inst {
                num_renamed += 1;
                insts.push(inst);
            }
        }

        let next_fetch_decode = if should_stall {
            Some(stages::wide::FetchDecode {
                insts: pipe.fetch_decode.insts[num_renamed..].to_vec(),
                next_pcs: pipe.fetch_decode.next_pcs[num_renamed..].to_vec(),
                stalled: false,
            })
        } else {
            None
        };

        stages::wide::Rename {
            insts,
            next_fetch_decode,
        }
    }

    fn rename_one(&mut self, inst: &Tagged<Inst>) -> stages::narrow::Rename {
        let mut stall = false;

        let Tagged { inst, tag } = inst.clone();

        if self.rob.is_full() {
            self.stats.rob_stalls += 1;
            stall = true;
        }
        if self.reservation_station.is_full() {
            self.stats.reservation_station_stalls += 1;
            stall = true;
        }
        if inst.is_mem_access() && !self.lsq.has_space(&inst) {
            self.stats.lsq_stalls += 1;
            stall = true;
        }

        if stall {
            return stages::narrow::Rename {
                inst: None,
                should_stall: true,
            };
        }

        if let Some(renamed_inst) = self.reg_file.perform_rename(tag, inst.clone()) {
            assert_eq!(self.rob.try_push(tag, inst), None);
            self.reservation_station.insert(tag, renamed_inst.clone());

            if renamed_inst.is_mem_access() {
                self.lsq.insert_access(renamed_inst.clone(), tag);
            }

            stages::narrow::Rename {
                inst: Some(renamed_inst),
                should_stall: false,
            }
        } else {
            self.stats.phys_reg_stalls += 1;
            stages::narrow::Rename {
                inst: None,
                should_stall: true,
            }
        }
    }

    fn stage_issue(&mut self, _pipe: &Pipeline) -> stages::wide::Issue {
        let mut remove_tags = vec![];
        let mut kill_tags = Vec::new();
        let mut reinsert_insts = Vec::new();

        for (tag, ready_inst) in self.reservation_station.get_ready(&self.reg_file) {
            if ready_inst.is_load()
                && !self.lsq.can_execute_load(
                    *tag,
                    *self
                        .pc_map
                        .get(tag)
                        .unwrap_or_else(|| panic!("no tag {:?}", tag)),
                    ready_inst.access_range(),
                    &mut self.reg_file,
                )
            {
                continue;
            } else if ready_inst.is_store() {
                let (eu_kills, mispredicts) =
                    self.lsq
                        .store_addr_known(*tag, ready_inst.access_range(), &mut self.reg_file);

                for tag in eu_kills {
                    // Kill it from the EUs (but this isn't a misprediction)
                    for eu in &mut self.execution_units {
                        if eu.eu_type == EuType::LoadStore {
                            self.lsq.kill_inflight(tag);
                            let killed = eu.kill_specific(tag);
                            reinsert_insts.push(killed);
                        }
                    }
                }

                // dbg!(&mispredicts);
                // assert!(mispredicts.len() <= 1);
                kill_tags = mispredicts;
            }

            if let Some(eu) = self
                .execution_units
                .iter_mut()
                .find(|eu| eu.can_execute(ready_inst))
            {
                if ready_inst.is_load() {
                    self.lsq.begin_execute_load(*tag);
                }

                eu.begin_execute(ready_inst.clone(), *tag);
                remove_tags.push(*tag);
            }

            if !kill_tags.is_empty() {
                break;
            }
        }

        for tag in remove_tags {
            self.reservation_station.pop_ready(tag);
        }

        for tagged in reinsert_insts {
            self.reservation_station
                .insert_ready(tagged.tag, tagged.inst);
        }

        // we should roll back to the earliest mis-speculation
        // kill_tags.iter().filter_map(|t| self.reg_file.end_predict_mem(*t, false))
        kill_tags.sort();

        let mut next_fetch = None;
        for tag in kill_tags.iter().rev() {
            next_fetch = self.reg_file.end_predict_mem(*tag, false);
        }

        for tag in kill_tags {
            // println!("KILLING TAGS AFTER {:?}", tag);
            self.kill_tags_after(Tag(tag.0 - 1));
        }

        stages::wide::Issue { next_fetch }
    }

    // Advance execution of all the execution units.
    fn stage_execute(&mut self, _pipe: &Pipeline) {
        for eu in &mut self.execution_units {
            eu.advance(&mut self.mem, &mut self.stats);
        }
    }

    fn stage_writeback(&mut self, pipe: &Pipeline) -> stages::wide::Writeback {
        let mut completed = vec![];
        let mut next_fetch = None;

        for _ in 0..PIPE_WIDTH {
            let mut res = self.writeback_one(pipe);
            if let Some(tagged) = res.inst.take() {
                completed.push(tagged);
            }

            // This will only happen once per cycle because there is only one branch unit
            // Otherwise we will need to recover to the oldest mis-speculated branch.
            if let Some(next_pc) = res.next_fetch.take() {
                debug_assert!(next_fetch.is_none());
                next_fetch = Some(next_pc);
            }
        }

        stages::wide::Writeback {
            insts: completed,
            next_fetch,
        }
    }

    // Take output of the execution units and write into the register file.
    fn writeback_one(&mut self, _pipe: &Pipeline) -> stages::narrow::Writeback {
        for eu in &mut self.execution_units {
            if let Some((Tagged { tag, inst }, result)) = eu.take_complete() {
                let mut next_fetch = None;

                match &inst {
                    Inst::Add(dst, _, _)
                    | Inst::AddImm(dst, _, _)
                    | Inst::Sub(dst, _, _)
                    | Inst::Mul(dst, _, _)
                    | Inst::MulHU(dst, _, _)
                    | Inst::Rem(dst, _, _)
                    | Inst::Div(dst, _, _)
                    | Inst::DivU(dst, _, _)
                    | Inst::And(dst, _, _)
                    | Inst::AndImm(dst, _, _)
                    | Inst::Or(dst, _, _)
                    | Inst::OrImm(dst, _, _)
                    | Inst::Xor(dst, _, _)
                    | Inst::XorImm(dst, _, _)
                    | Inst::LoadFullImm(dst, _)
                    | Inst::LoadUpperImm(dst, _)
                    | Inst::SetLessThanU(dst, _, _)
                    | Inst::SetLessThanImm(dst, _, _)
                    | Inst::SetLessThanImmU(dst, _, _)
                    | Inst::ShiftLeftLogicalImm(dst, _, _)
                    | Inst::ShiftRightArithImm(dst, _, _)
                    | Inst::ShiftRightLogicalImm(dst, _, _)
                    | Inst::EffectiveAddress(dst, _, _, _)
                    | Inst::IndexedLoadByteU(dst, _, _, _)
                    | Inst::LoadWord(dst, _)
                    | Inst::LoadHalfWord(dst, _)
                    | Inst::LoadByte(dst, _)
                    | Inst::LoadByteU(dst, _) => {
                        if inst.is_load() {
                            self.lsq.writeback_load(tag);
                            self.pc_map.remove(&tag);
                        }

                        if dst.arch != ArchReg::Zero {
                            self.reg_file.set_phys_active(dst.phys, result.val);
                        }
                    }
                    Inst::BranchIfEqual(_, _, _)
                    | Inst::BranchIfLess(_, _, _)
                    | Inst::BranchIfLessU(_, _, _)
                    | Inst::BranchIfNotEqual(_, _, _)
                    | Inst::BranchIfGreaterEqualU(_, _, _)
                    | Inst::BranchIfGreaterEqual(_, _, _) => {
                        let taken = result.val == 1;
                        let predicted_taken = self.reg_file.was_predicted_taken(tag);

                        self.stats.direct_predicts += 1;

                        if let Some(next_pc) = self.reg_file.end_predict_direct(
                            tag,
                            taken,
                            predicted_taken,
                            &mut self.branch_predictor,
                        ) {
                            // Flush
                            // println!("FLUSH DIRECT");
                            self.stats.direct_mispredicts += 1;
                            self.kill_tags_after(tag);
                            next_fetch = Some(next_pc);
                        }
                    }
                    Inst::JumpAndLink(dst, _) => {
                        let inst_pc = self.pc_map.remove(&tag).unwrap();

                        if dst.arch != ArchReg::Zero {
                            self.reg_file
                                .set_phys_active(dst.phys, (inst_pc + INST_SIZE).0);
                        }
                    }
                    Inst::JumpAndLinkRegister(dst, _, _) => {
                        let (predicted_pc, inst_pc) = self.reg_file.predicted_addr(tag);

                        if predicted_pc.is_some() {
                            self.stats.indirect_predicts += 1;
                        }

                        if dst.arch != ArchReg::Zero {
                            self.reg_file
                                .set_phys_active(dst.phys, (inst_pc + INST_SIZE).0);
                        }

                        let actual_pc = AbsPc(result.val);
                        self.reg_file.end_predict_indirect(
                            tag,
                            actual_pc,
                            predicted_pc,
                            &mut self.branch_predictor,
                        );
                        if predicted_pc
                            .map(|predicted_pc| actual_pc != predicted_pc)
                            .unwrap_or(false)
                        {
                            // println!("FLUSH INDIRECT");
                            self.stats.indirect_mispredicts += 1;
                            self.kill_tags_after(tag);
                            next_fetch = Some(actual_pc);
                        } else if predicted_pc.is_none() {
                            next_fetch = Some(actual_pc);
                        }
                    }
                    Inst::StoreWord(_, _)
                    | Inst::StoreHalfWord(_, _)
                    | Inst::StoreByte(_, _)
                    | Inst::Halt => (),
                    // _ => unimplemented!("{:?}", inst),
                };

                self.rob.mark_complete(tag);

                return stages::narrow::Writeback {
                    inst: Some(Tagged { tag, inst }),
                    next_fetch,
                };
            }
        }

        stages::narrow::Writeback {
            inst: None,
            next_fetch: None,
        }
    }

    // Commit instructions from the ROB to architectural state.
    fn stage_commit(&mut self, _pipe: &Pipeline) -> stages::Commit {
        for _ in 0..PIPE_WIDTH {
            let tagged = self.rob.try_pop();

            let Tagged { tag, inst } = match tagged {
                Some(Tagged {
                    inst: Inst::Halt, ..
                }) => return stages::Commit { should_halt: true },
                Some(tagged) => tagged,
                None => return Default::default(),
            };

            match inst {
                Inst::Add(dst, _, _)
                | Inst::AddImm(dst, _, _)
                | Inst::Sub(dst, _, _)
                | Inst::Mul(dst, _, _)
                | Inst::MulHU(dst, _, _)
                | Inst::Rem(dst, _, _)
                | Inst::Div(dst, _, _)
                | Inst::DivU(dst, _, _)
                | Inst::And(dst, _, _)
                | Inst::AndImm(dst, _, _)
                | Inst::Or(dst, _, _)
                | Inst::OrImm(dst, _, _)
                | Inst::Xor(dst, _, _)
                | Inst::XorImm(dst, _, _)
                | Inst::ShiftRightLogicalImm(dst, _, _)
                | Inst::ShiftRightArithImm(dst, _, _)
                | Inst::ShiftLeftLogicalImm(dst, _, _)
                | Inst::SetLessThanU(dst, _, _)
                | Inst::SetLessThanImm(dst, _, _)
                | Inst::SetLessThanImmU(dst, _, _)
                | Inst::JumpAndLink(dst, _)
                | Inst::JumpAndLinkRegister(dst, _, _)
                | Inst::EffectiveAddress(dst, _, _, _)
                | Inst::IndexedLoadByteU(dst, _, _, _)
                | Inst::LoadFullImm(dst, _)
                | Inst::LoadUpperImm(dst, _)
                | Inst::LoadByteU(dst, _)
                | Inst::LoadByte(dst, _)
                | Inst::LoadHalfWord(dst, _)
                | Inst::LoadWord(dst, _) => {
                    if dst != ArchReg::Zero {
                        self.reg_file.release_phys(tag);
                    }

                    if inst.is_mem_access() {
                        // If we got to this point, the speculation was correct.
                        self.reg_file.end_predict_mem(tag, true);
                        self.lsq.release_load(tag);
                    }
                }
                Inst::StoreByte(_, _) | Inst::StoreWord(_, _) => {
                    self.lsq.commit_store(tag, &self.reg_file, &mut self.mem)
                }
                Inst::BranchIfEqual(_, _, _)
                | Inst::BranchIfLess(_, _, _)
                | Inst::BranchIfLessU(_, _, _)
                | Inst::BranchIfNotEqual(_, _, _)
                | Inst::BranchIfGreaterEqual(_, _, _)
                | Inst::BranchIfGreaterEqualU(_, _, _) => (),
                _ => unimplemented!("{:?}", inst),
            }

            self.stats.insts_retired += 1;

            if inst.is_fused() {
                self.stats.macro_ops_fused += 1;
            }
        }

        stages::Commit { should_halt: false }
    }

    fn kill_tags_after(&mut self, tag: Tag) {
        for eu in &mut self.execution_units {
            eu.kill_tags_after(tag);
        }

        self.pc_map.retain(|t, _| *t <= tag);

        self.reservation_station.kill_tags_after(tag);
        self.lsq.kill_tags_after(tag);
        self.rob.kill_tags_after(tag);
        self.reg_file.kill_tags_after(tag);
    }
}
