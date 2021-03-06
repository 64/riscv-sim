use crate::{
    execution_unit::EuType,
    regs::{PrfEntry, RegFile},
    util::Addr,
};

use std::{
    fmt::{self, Debug},
    ops::{Add, AddAssign, Range, Sub},
    str::FromStr,
};
use strum::{self, EnumIter, EnumString};

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Imm(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub String);

#[derive(Debug, Copy, Default, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct AbsPc(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MemRef<RegType: Debug + Clone = ArchReg> {
    pub base: RegType,
    pub offset: Imm,
}

pub const INST_SIZE: u32 = 4;

// https://en.wikichip.org/wiki/risc-v/registers
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum ArchReg {
    Zero,
    RA,
    SP,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
}

// https://mark.theis.site/riscv/
// https://web.eecs.utk.edu/~smarz1/courses/ece356/notes/assembly/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst<
    SrcReg: Debug + Clone = ArchReg,
    DstReg: Debug + Clone = ArchReg,
    JumpType: Debug + Clone = AbsPc,
> {
    LoadByte(DstReg, MemRef<SrcReg>),
    LoadByteU(DstReg, MemRef<SrcReg>),
    LoadHalfWord(DstReg, MemRef<SrcReg>),
    LoadWord(DstReg, MemRef<SrcReg>),
    IndexedLoadByteU(DstReg, SrcReg, SrcReg, Imm),
    EffectiveAddress(DstReg, SrcReg, SrcReg, Imm),
    LoadFullImm(DstReg, Imm),
    StoreByte(SrcReg, MemRef<SrcReg>),
    StoreHalfWord(SrcReg, MemRef<SrcReg>),
    StoreWord(SrcReg, MemRef<SrcReg>),
    LoadUpperImm(DstReg, Imm),
    Add(DstReg, SrcReg, SrcReg),
    AddImm(DstReg, SrcReg, Imm),
    Sub(DstReg, SrcReg, SrcReg),
    Or(DstReg, SrcReg, SrcReg),
    OrImm(DstReg, SrcReg, Imm),
    Xor(DstReg, SrcReg, SrcReg),
    XorImm(DstReg, SrcReg, Imm),
    And(DstReg, SrcReg, SrcReg),
    AndImm(DstReg, SrcReg, Imm),
    ShiftLeftLogicalImm(DstReg, SrcReg, Imm),
    ShiftRightArithImm(DstReg, SrcReg, Imm),
    ShiftRightLogicalImm(DstReg, SrcReg, Imm),
    Mul(DstReg, SrcReg, SrcReg),
    MulHU(DstReg, SrcReg, SrcReg),
    Rem(DstReg, SrcReg, SrcReg),
    Div(DstReg, SrcReg, SrcReg),
    DivU(DstReg, SrcReg, SrcReg),
    JumpAndLink(DstReg, JumpType),
    JumpAndLinkRegister(DstReg, SrcReg, Imm),
    BranchIfEqual(SrcReg, SrcReg, JumpType),
    BranchIfNotEqual(SrcReg, SrcReg, JumpType),
    BranchIfGreaterEqual(SrcReg, SrcReg, JumpType),
    BranchIfGreaterEqualU(SrcReg, SrcReg, JumpType),
    BranchIfLess(SrcReg, SrcReg, JumpType),
    BranchIfLessU(SrcReg, SrcReg, JumpType),
    SetLessThanImm(DstReg, SrcReg, Imm),
    SetLessThanU(DstReg, SrcReg, SrcReg),
    SetLessThanImmU(DstReg, SrcReg, Imm),
    Halt, // Used internally when execution finishes.
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(pub u64);

#[derive(Debug, Clone)]
pub struct Tagged<I> {
    pub tag: Tag,
    pub inst: I,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PhysReg(i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueOrReg {
    Value(u32),
    Reg(PhysReg),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BothReg {
    pub arch: ArchReg,
    pub phys: PhysReg,
}

// Inst with labels not yet resolved to PC values.
pub type LabeledInst = Inst<ArchReg, ArchReg, Label>;

// Inst with its source operands ready for computation.
pub type ReadyInst = Inst<u32, BothReg>;

// Inst with its source operands renamed.
pub type RenamedInst = Inst<ValueOrReg, BothReg>;

// Inst after execution. The source registers are no longer used.
pub type ExecutedInst = Inst<(), BothReg>;

impl FromStr for LabeledInst {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (op, args) = s.split_once(' ').unwrap_or((s, ""));
        let args = args.split(',').collect::<Vec<_>>();

        let nth_arg = |n: usize| -> Result<&str, String> {
            args.get(n)
                .map(|s| s.trim())
                .and_then(|s| if s.is_empty() { None } else { Some(s) })
                .ok_or_else(|| format!("cannot fetch argument {n}"))
        };
        let mem_arg = |n: usize| -> Result<MemRef, String> { MemRef::from_str(nth_arg(n)?) };
        let imm_arg = |n: usize| -> Result<Imm, String> { Imm::from_str(nth_arg(n)?) };
        let label_arg = |n: usize| -> Result<Label, String> { Label::from_str(nth_arg(n)?) };
        let reg_arg = |n: usize| -> Result<ArchReg, String> {
            ArchReg::from_str(nth_arg(n)?).map_err(|e| e.to_string())
        };

        #[rustfmt::skip]
        let inst = match op.to_lowercase().as_str() {
            "lb" => LabeledInst::LoadByte(reg_arg(0)?, mem_arg(1)?),
            "lbu" => LabeledInst::LoadByteU(reg_arg(0)?, mem_arg(1)?),
            "lh" => LabeledInst::LoadHalfWord(reg_arg(0)?, mem_arg(1)?),
            "lw" => LabeledInst::LoadWord(reg_arg(0)?, mem_arg(1)?),
            "sb" => LabeledInst::StoreByte(reg_arg(0)?, mem_arg(1)?),
            "sh" => LabeledInst::StoreHalfWord(reg_arg(0)?, mem_arg(1)?),
            "sw" => LabeledInst::StoreWord(reg_arg(0)?, mem_arg(1)?),
            "add" => LabeledInst::Add(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "addi" => LabeledInst::AddImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "sub" => LabeledInst::Sub(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "neg" => LabeledInst::Sub(reg_arg(0)?, ArchReg::Zero, reg_arg(1)?),
            "and" => LabeledInst::And(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "andi" => LabeledInst::AndImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "or" => LabeledInst::Or(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "ori" => LabeledInst::OrImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "xori" => LabeledInst::XorImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "xor" => LabeledInst::Xor(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "slli" => LabeledInst::ShiftLeftLogicalImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "srai" => LabeledInst::ShiftRightArithImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "srli" => LabeledInst::ShiftRightLogicalImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "mul" => LabeledInst::Mul(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "mulhu" => LabeledInst::MulHU(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "rem" => LabeledInst::Rem(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "div" => LabeledInst::Div(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "divu" => LabeledInst::DivU(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "li" => LabeledInst::AddImm(reg_arg(0)?, ArchReg::Zero, imm_arg(1)?),
            "lui" => LabeledInst::LoadUpperImm(reg_arg(0)?, imm_arg(1)?),
            "mv" => LabeledInst::AddImm(reg_arg(0)?, reg_arg(1)?, Imm(0)),
            "j" => LabeledInst::JumpAndLink(ArchReg::Zero, label_arg(0)?),
            "jal" => LabeledInst::JumpAndLink(reg_arg(0)?, label_arg(1)?),
            "call" => LabeledInst::JumpAndLink(ArchReg::RA, label_arg(0)?),
            "jr" => LabeledInst::JumpAndLinkRegister(ArchReg::Zero, reg_arg(0)?, Imm(0)),
            "ret" => LabeledInst::JumpAndLinkRegister(ArchReg::Zero, ArchReg::RA, Imm(0)),
            "beq" => LabeledInst::BranchIfEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "beqz" => LabeledInst::BranchIfEqual(reg_arg(0)?, ArchReg::Zero, label_arg(1)?),
            "bne" => LabeledInst::BranchIfNotEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bnez" => LabeledInst::BranchIfNotEqual(reg_arg(0)?, ArchReg::Zero, label_arg(1)?),
            "bge" => LabeledInst::BranchIfGreaterEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "ble" => LabeledInst::BranchIfGreaterEqual(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
            "blez" => LabeledInst::BranchIfGreaterEqual(ArchReg::Zero, reg_arg(0)?, label_arg(1)?),
            "bgez" => LabeledInst::BranchIfGreaterEqual(reg_arg(0)?, ArchReg::Zero, label_arg(1)?),
            "blt" => LabeledInst::BranchIfLess(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bltu" => LabeledInst::BranchIfLessU(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bgt" => LabeledInst::BranchIfLess(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
            "bleu" => LabeledInst::BranchIfGreaterEqualU(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
            "bgeu" => LabeledInst::BranchIfGreaterEqualU(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bgtu" => LabeledInst::BranchIfLessU(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
            "sltiu" => LabeledInst::SetLessThanImmU(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "snez" => LabeledInst::SetLessThanU(reg_arg(0)?, ArchReg::Zero, reg_arg(1)?),
            "slti" => LabeledInst::SetLessThanImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "seqz" => LabeledInst::SetLessThanImmU(reg_arg(0)?, reg_arg(1)?, Imm(1)),
            "hlt" => LabeledInst::Halt,
            "nop" => LabeledInst::nop(),
            _ => return Err(format!("unknown instruction: '{}'", op)),
        };

        Ok(inst)
    }
}

impl<J: Debug + Clone> Inst<ArchReg, ArchReg, J> {
    pub fn nop() -> Self {
        Inst::AddImm(ArchReg::Zero, ArchReg::Zero, Imm(0))
    }
}

impl<SrcReg: Debug + Clone, DstReg: Debug + Clone, JumpType: Debug + Clone>
    Inst<SrcReg, DstReg, JumpType>
{
    pub fn is_branch(&self) -> bool {
        matches!(
            self,
            Inst::JumpAndLink(_, _)
                | Inst::JumpAndLinkRegister(_, _, _)
                | Inst::BranchIfNotEqual(_, _, _)
                | Inst::BranchIfEqual(_, _, _)
                | Inst::BranchIfLess(_, _, _)
                | Inst::BranchIfLessU(_, _, _)
                | Inst::BranchIfGreaterEqual(_, _, _)
                | Inst::BranchIfGreaterEqualU(_, _, _)
        )
    }

    pub fn is_mem_access(&self) -> bool {
        self.eu_type() == EuType::LoadStore
    }

    pub fn is_fused(&self) -> bool {
        matches!(
            self,
            Inst::EffectiveAddress(_, _, _, _)
                | Inst::IndexedLoadByteU(_, _, _, _)
                | Inst::LoadFullImm(_, _)
        )
    }

    pub fn is_load(&self) -> bool {
        matches!(
            self,
            Inst::LoadByte(_, _)
                | Inst::LoadByteU(_, _)
                | Inst::LoadHalfWord(_, _)
                | Inst::LoadWord(_, _)
                | Inst::IndexedLoadByteU(_, _, _, _)
        )
    }

    pub fn is_store(&self) -> bool {
        matches!(
            self,
            Inst::StoreByte(_, _) | Inst::StoreHalfWord(_, _) | Inst::StoreWord(_, _)
        )
    }

    pub fn eu_type(&self) -> EuType {
        match self {
            Inst::JumpAndLink(_, _)
            | Inst::JumpAndLinkRegister(_, _, _)
            | Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfLess(_, _, _)
            | Inst::BranchIfLessU(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _)
            | Inst::BranchIfGreaterEqualU(_, _, _) => EuType::Branch,
            Inst::Add(_, _, _)
            | Inst::AddImm(_, _, _)
            | Inst::Sub(_, _, _)
            | Inst::Or(_, _, _)
            | Inst::OrImm(_, _, _)
            | Inst::Xor(_, _, _)
            | Inst::XorImm(_, _, _)
            | Inst::And(_, _, _)
            | Inst::AndImm(_, _, _)
            | Inst::EffectiveAddress(_, _, _, _)
            | Inst::ShiftLeftLogicalImm(_, _, _)
            | Inst::ShiftRightArithImm(_, _, _)
            | Inst::ShiftRightLogicalImm(_, _, _)
            | Inst::SetLessThanImmU(_, _, _)
            | Inst::SetLessThanU(_, _, _)
            | Inst::SetLessThanImm(_, _, _)
            | Inst::LoadUpperImm(_, _)
            | Inst::LoadFullImm(_, _)
            | Inst::Mul(_, _, _)
            | Inst::MulHU(_, _, _)
            | Inst::DivU(_, _, _)
            | Inst::Div(_, _, _)
            | Inst::Rem(_, _, _) => EuType::Alu,
            Inst::LoadByte(_, _)
            | Inst::LoadByteU(_, _)
            | Inst::LoadHalfWord(_, _)
            | Inst::LoadWord(_, _)
            | Inst::IndexedLoadByteU(_, _, _, _)
            | Inst::StoreByte(_, _)
            | Inst::StoreHalfWord(_, _)
            | Inst::StoreWord(_, _) => EuType::LoadStore,
            Inst::Halt => EuType::Special,
        }
    }

    pub fn latency(&self) -> u64 {
        match self {
            x if x.is_branch() => 1,
            x if x.is_mem_access() => 3,
            Inst::Add(_, _, _)
            | Inst::Sub(_, _, _)
            | Inst::SetLessThanU(_, _, _)
            | Inst::SetLessThanImm(_, _, _)
            | Inst::SetLessThanImmU(_, _, _)
            | Inst::AddImm(_, _, _)
            | Inst::And(_, _, _)
            | Inst::AndImm(_, _, _)
            | Inst::Or(_, _, _)
            | Inst::OrImm(_, _, _)
            | Inst::Xor(_, _, _)
            | Inst::XorImm(_, _, _)
            | Inst::LoadFullImm(_, _)
            | Inst::LoadUpperImm(_, _)
            | Inst::EffectiveAddress(_, _, _, _)
            | Inst::ShiftRightArithImm(_, _, _)
            | Inst::ShiftRightLogicalImm(_, _, _)
            | Inst::ShiftLeftLogicalImm(_, _, _) => 1,
            Inst::Mul(_, _, _) => 2,
            Inst::Rem(_, _, _) | Inst::Div(_, _, _) | Inst::DivU(_, _, _) => 3,
            Inst::Halt => 1,
            _ => unimplemented!("{:?}", self),
        }
    }

    #[rustfmt::skip]
    pub fn try_map<OtherSrcReg, OtherDstReg, OtherJumpType, SrcFn, DstFn, JumpFn>(
        self,
        mut src_fn: SrcFn,
        mut dst_fn: DstFn,
        mut jump_fn: JumpFn,
    ) -> Option<Inst<OtherSrcReg, OtherDstReg, OtherJumpType>>
    where
        OtherSrcReg: Debug + Clone,
        OtherDstReg: Debug + Clone,
        OtherJumpType:  Debug + Clone,
        SrcFn: FnMut(SrcReg) -> Option<OtherSrcReg>,
        DstFn: FnMut(DstReg) -> Option<OtherDstReg>,
        JumpFn: FnMut(JumpType) -> Option<OtherJumpType>,
    {
        Some(match self {
            Inst::Add(dst, src0, src1) => Inst::Add(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::AddImm(dst, src, imm) => Inst::AddImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::Sub(dst, src0, src1) => Inst::Sub(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::And(dst, src0, src1) => Inst::And(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::AndImm(dst, src, imm) => Inst::AndImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::Or(dst, src0, src1) => Inst::Or(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::OrImm(dst, src, imm) => Inst::OrImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::Xor(dst, src0, src1) => Inst::Xor(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::XorImm(dst, src, imm) => Inst::XorImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::LoadFullImm(dst, imm) => Inst::LoadFullImm(dst_fn(dst)?, imm),
            Inst::LoadUpperImm(dst, imm) => Inst::LoadUpperImm(dst_fn(dst)?, imm),
            Inst::EffectiveAddress(dst, src1, src2, imm) => Inst::EffectiveAddress(dst_fn(dst)?, src_fn(src1)?, src_fn(src2)?, imm),
            Inst::ShiftLeftLogicalImm(dst, src, imm) => Inst::ShiftLeftLogicalImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::ShiftRightArithImm(dst, src, imm) => Inst::ShiftRightArithImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::ShiftRightLogicalImm(dst, src, imm) => Inst::ShiftRightLogicalImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::SetLessThanU(dst, src1, src2) => Inst::SetLessThanU(dst_fn(dst)?, src_fn(src1)?, src_fn(src2)?),
            Inst::SetLessThanImm(dst, src, imm) => Inst::SetLessThanImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::SetLessThanImmU(dst, src, imm) => Inst::SetLessThanImmU(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::Mul(dst, src0, src1) => Inst::Mul(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::MulHU(dst, src0, src1) => Inst::MulHU(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::Rem(dst, src0, src1) => Inst::Rem(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::Div(dst, src0, src1) => Inst::Div(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::DivU(dst, src0, src1) => Inst::DivU(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::LoadByte(dst, src) => Inst::LoadByte(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::LoadByteU(dst, src) => Inst::LoadByteU(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::LoadHalfWord(dst, src) => Inst::LoadHalfWord(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::LoadWord(dst, src) => Inst::LoadWord(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::IndexedLoadByteU(dst, src0, src1, imm) => Inst::IndexedLoadByteU(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?, imm),
            Inst::StoreByte(src, dst) => Inst::StoreByte(src_fn(src)?, MemRef { base: src_fn(dst.base)?, offset: dst.offset }),
            Inst::StoreHalfWord(src, dst) => Inst::StoreHalfWord(src_fn(src)?, MemRef { base: src_fn(dst.base)?, offset: dst.offset }),
            Inst::StoreWord(src, dst) => Inst::StoreWord(src_fn(src)?, MemRef { base: src_fn(dst.base)?, offset: dst.offset }),
            Inst::JumpAndLink(dst, label) => Inst::JumpAndLink(dst_fn(dst)?, jump_fn(label)?),
            Inst::JumpAndLinkRegister(dst, src, imm) => Inst::JumpAndLinkRegister(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::BranchIfNotEqual(src0, src1, label) => Inst::BranchIfNotEqual(src_fn(src0)?, src_fn(src1)?, jump_fn(label)?),
            Inst::BranchIfEqual(src0, src1, label) => Inst::BranchIfEqual(src_fn(src0)?, src_fn(src1)?, jump_fn(label)?),
            Inst::BranchIfGreaterEqual(src0, src1, label)=> Inst::BranchIfGreaterEqual(src_fn(src0)?, src_fn(src1)?, jump_fn(label)?),
            Inst::BranchIfGreaterEqualU(src0, src1, label)=> Inst::BranchIfGreaterEqualU(src_fn(src0)?, src_fn(src1)?, jump_fn(label)?),
            Inst::BranchIfLess(src0, src1, label)=> Inst::BranchIfLess(src_fn(src0)?, src_fn(src1)?, jump_fn(label)?),
            Inst::BranchIfLessU(src0, src1, label)=> Inst::BranchIfLessU(src_fn(src0)?, src_fn(src1)?, jump_fn(label)?),
            Inst::Halt => Inst::Halt,
        })
    }

    pub fn map_regs<OtherSrcReg, OtherDstReg, SrcFn, DstFn>(
        self,
        mut src_fn: SrcFn,
        mut dst_fn: DstFn,
    ) -> Inst<OtherSrcReg, OtherDstReg, JumpType>
    where
        OtherSrcReg: Debug + Clone,
        OtherDstReg: Debug + Clone,
        SrcFn: FnMut(SrcReg) -> OtherSrcReg,
        DstFn: FnMut(DstReg) -> OtherDstReg,
    {
        self.try_map(
            |src_reg| Some(src_fn(src_reg)),
            |dst_reg| Some(dst_fn(dst_reg)),
            |jmp| Some(jmp),
        )
        .unwrap()
    }

    pub fn map_src_regs<OtherSrcReg, SrcFn>(
        self,
        src_fn: SrcFn,
    ) -> Inst<OtherSrcReg, DstReg, JumpType>
    where
        OtherSrcReg: Debug + Clone,
        SrcFn: FnMut(SrcReg) -> OtherSrcReg,
    {
        self.map_regs(src_fn, |dst_reg| dst_reg)
    }

    // pub fn map_dst_regs<OtherDstReg, DstFn>(
    //     self,
    //     mut dst_fn: DstFn,
    // ) -> Inst<SrcReg, OtherDstReg, JumpType>
    // where
    //     OtherDstReg: Debug + Clone,
    //     DstFn: FnMut(DstReg) -> OtherDstReg,
    // {
    //     self.map_regs(|src_reg| src_reg, |dst_reg| dst_fn(dst_reg))
    // }

    pub fn map_jumps<OtherJumpType, JumpFn>(
        self,
        mut jump_fn: JumpFn,
    ) -> Inst<SrcReg, DstReg, OtherJumpType>
    where
        OtherJumpType: Debug + Clone,
        JumpFn: FnMut(JumpType) -> OtherJumpType,
    {
        self.try_map(
            |src_reg| Some(src_reg),
            |dst_reg| Some(dst_reg),
            |jump| Some(jump_fn(jump)),
        )
        .unwrap()
    }

    pub fn executed(self) -> Inst<(), DstReg, JumpType> {
        self.map_regs(|_src_reg| (), |dst_reg| dst_reg)
    }
}

impl RenamedInst {
    pub fn get_ready(&self, rf: &RegFile) -> Option<ReadyInst> {
        self.clone().try_map(
            |r| match r {
                ValueOrReg::Value(x) => Some(x),
                ValueOrReg::Reg(phys_reg) => match rf.get_phys(phys_reg) {
                    PrfEntry::Active(x) => Some(x),
                    _ => None,
                },
            },
            Some,
            Some,
        )
    }
}

impl ReadyInst {
    pub fn access_addr(&self) -> Addr {
        match self {
            Inst::LoadWord(_, dst)
            | Inst::LoadHalfWord(_, dst)
            | Inst::LoadByte(_, dst)
            | Inst::LoadByteU(_, dst)
            | Inst::StoreWord(_, dst)
            | Inst::StoreHalfWord(_, dst)
            | Inst::StoreByte(_, dst) => dst.compute_addr(),
            Inst::IndexedLoadByteU(_, src1, src2, imm) => Addr(src1 + src2 + imm.0),
            _ => unimplemented!("{:?}", self),
        }
    }

    pub fn access_range(&self) -> Range<u32> {
        let start = self.access_addr().0;
        let end = match self {
            Inst::StoreWord(_, _) | Inst::LoadWord(_, _) => start + 4,
            Inst::StoreHalfWord(_, _) | Inst::LoadHalfWord(_, _) => start + 2,
            Inst::LoadByte(_, _)
            | Inst::LoadByteU(_, _)
            | Inst::StoreByte(_, _)
            | Inst::IndexedLoadByteU(_, _, _, _) => start + 1,
            _ => unimplemented!("{:?}", self),
        };

        start..end
    }
}

impl FromStr for Imm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let val = if let Some(s) = s.strip_prefix("0x") {
            i64::from_str_radix(s, 16)
        } else if let Some(s) = s.strip_prefix("-0x") {
            i64::from_str_radix(s, 16).map(|v| -v)
        } else {
            i64::from_str(s)
        };

        let val = val.map_err(|_| format!("invalid immediate: '{s}'"))?;

        if let Ok(u) = u32::try_from(val) {
            Ok(Self(u))
        } else if let Ok(s) = i32::try_from(val) {
            assert!(s < 0);
            let val = u32::from_le_bytes(s.to_le_bytes());
            Ok(Self(val))
        } else {
            Err(format!("invalid immediate: '{s}'"))
        }
    }
}

impl FromStr for Label {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().all(|c| c.is_alphanumeric() || "_.".contains(c)) {
            true => Ok(Label(s.to_owned())),
            false => Err(format!("invalid label name: '{s}'")),
        }
    }
}

impl FromStr for MemRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (outer, rest) = s
            .split_once('(')
            .ok_or_else(|| format!("invalid memory reference (expected '('): '{s}'"))?;
        let (inner, rest) = rest
            .split_once(')')
            .ok_or_else(|| format!("invalid memory reference (expected ')'): '{s}'"))?;

        if !rest.trim().is_empty() {
            return Err(format!(
                "invalid memory reference (unexpected suffix): '{s}'"
            ));
        }

        let base = inner
            .parse::<ArchReg>()
            .map_err(|_| format!("invalid mem ref (reg): '{s}'"))?;
        let offset = outer
            .parse::<Imm>()
            .map_err(|_| format!("invalid mem ref (imm): '{s}'"))?;

        Ok(MemRef { base, offset })
    }
}

impl MemRef<u32> {
    pub fn compute_addr(self) -> Addr {
        Addr(self.base.wrapping_add(self.offset.0))
    }
}

impl Default for ArchReg {
    fn default() -> Self {
        ArchReg::Zero
    }
}

impl From<u64> for Tag {
    fn from(x: u64) -> Self {
        Self(x)
    }
}

impl fmt::Debug for Imm {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Imm({})", self.0)
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Tag({})", self.0)
    }
}

impl Default for PhysReg {
    fn default() -> Self {
        PhysReg::none()
    }
}

impl PhysReg {
    pub fn none() -> Self {
        Self(-1)
    }
}

impl fmt::Debug for PhysReg {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "PhysReg({})", self.0)
    }
}

impl From<i32> for PhysReg {
    fn from(x: i32) -> Self {
        Self(x)
    }
}

impl From<usize> for PhysReg {
    fn from(x: usize) -> Self {
        Self(x.try_into().unwrap())
    }
}

impl From<PhysReg> for usize {
    fn from(r: PhysReg) -> Self {
        r.0.try_into().expect("could not convert PhysReg to usize")
    }
}

impl Add<u32> for AbsPc {
    type Output = AbsPc;

    fn add(self, other: u32) -> AbsPc {
        AbsPc(self.0 + other)
    }
}

impl AddAssign<u32> for AbsPc {
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs
    }
}

impl Sub<u32> for AbsPc {
    type Output = AbsPc;

    fn sub(self, other: u32) -> AbsPc {
        AbsPc(self.0 - other)
    }
}

impl From<AbsPc> for u32 {
    fn from(pc: AbsPc) -> Self {
        pc.0
    }
}

impl From<u32> for AbsPc {
    fn from(pc: u32) -> Self {
        AbsPc(pc)
    }
}

impl TryFrom<usize> for AbsPc {
    type Error = <u32 as TryFrom<usize>>::Error;

    fn try_from(pc: usize) -> Result<Self, Self::Error> {
        Ok(AbsPc(pc.try_into()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reg() {
        assert_eq!(ArchReg::from_str("zero"), Ok(ArchReg::Zero));
        assert_eq!(ArchReg::from_str("sp"), Ok(ArchReg::SP));
        assert_eq!(ArchReg::from_str("ra"), Ok(ArchReg::RA));
        assert_eq!(ArchReg::from_str("a0"), Ok(ArchReg::A0));
        assert_eq!(ArchReg::from_str("a1"), Ok(ArchReg::A1));
        assert_eq!(ArchReg::from_str("a7"), Ok(ArchReg::A7));
        assert_eq!(ArchReg::from_str("t0"), Ok(ArchReg::T0));
        assert_eq!(ArchReg::from_str("t1"), Ok(ArchReg::T1));
        assert!(ArchReg::from_str("0").is_err());
        assert!(ArchReg::from_str("a-0").is_err());
        assert!(ArchReg::from_str("a-1").is_err());
        assert!(ArchReg::from_str("a50").is_err());
    }

    #[test]
    #[rustfmt::skip]
    fn test_memref() {
        assert_eq!(MemRef::from_str("0(a1)"), Ok(MemRef { offset: Imm(0), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("5(a1)"), Ok(MemRef { offset: Imm(5), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("-0(a1)"), Ok(MemRef { offset: Imm(0), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("-0(zero)"), Ok(MemRef { offset: Imm(0), base: ArchReg::Zero }));
        assert_eq!(MemRef::from_str("1(a1)"), Ok(MemRef { offset: Imm(1), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("0x123(a1)"), Ok(MemRef { offset: Imm(0x123), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("-1(a1)"), Ok(MemRef { offset: Imm(u32::MAX), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("-2(a1)"), Ok(MemRef { offset: Imm(u32::MAX - 1), base: ArchReg::A1 }));
        assert_eq!(MemRef::from_str("-0x123(a1)"), Ok(MemRef { offset: Imm(u32::MAX - 0x123 + 1), base: ArchReg::A1 }));

        assert!(MemRef::from_str("(a1)").is_err());
        assert!(MemRef::from_str("0").is_err());
        assert!(MemRef::from_str("a1(0)").is_err());
        assert!(MemRef::from_str("()").is_err());
    }

    #[test]
    #[rustfmt::skip]
    fn test_label() {
        assert_eq!(Label::from_str("foo"), Ok(Label("foo".to_string())));
        assert_eq!(Label::from_str(".foo"), Ok(Label(".foo".to_string())));
        assert_eq!(Label::from_str(".foo_bar"), Ok(Label(".foo_bar".to_string())));
        assert_eq!(Label::from_str(".foo_bar5"), Ok(Label(".foo_bar5".to_string())));
        assert_eq!(Label::from_str(".foo_BaR5"), Ok(Label(".foo_BaR5".to_string())));
        assert_eq!(Label::from_str("FOO_bar"), Ok(Label("FOO_bar".to_string())));

        assert_ne!(Label::from_str("foo"), Label::from_str("bar"));
        assert_ne!(Label::from_str("FOO_bar"), Label::from_str("foo_BAR"));

        assert!(Label::from_str("foo bar").is_err());
        assert!(Label::from_str("foo-bar").is_err());
        assert!(Label::from_str("[foobar").is_err());
        assert!(Label::from_str("foobar:").is_err());
    }
}
