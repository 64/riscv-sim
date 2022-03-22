use crate::{
    execution_unit::EuType,
    regs::{PrfEntry, RegFile},
    util::Addr,
};
use lru_mem::HeapSize;
use std::{
    fmt::{self, Debug},
    str::FromStr,
};
use strum::{self, EnumIter, EnumString};

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Imm(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MemRef<RegType: Debug + Clone = ArchReg> {
    pub base: RegType,
    pub offset: Imm,
}

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
}

// https://mark.theis.site/riscv/
// https://web.eecs.utk.edu/~smarz1/courses/ece356/notes/assembly/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst<SrcReg: Debug + Clone = ArchReg, DstReg: Debug + Clone = ArchReg> {
    LoadByte(DstReg, MemRef<SrcReg>),
    LoadHalfWord(DstReg, MemRef<SrcReg>),
    LoadWord(DstReg, MemRef<SrcReg>),
    StoreByte(SrcReg, MemRef<SrcReg>),
    StoreHalfWord(SrcReg, MemRef<SrcReg>),
    StoreWord(SrcReg, MemRef<SrcReg>),
    Add(DstReg, SrcReg, SrcReg),
    AddImm(DstReg, SrcReg, Imm),
    AndImm(DstReg, SrcReg, Imm),
    ShiftLeftLogicalImm(DstReg, SrcReg, Imm),
    Rem(DstReg, SrcReg, SrcReg),
    Jump(Label),
    JumpAndLink(DstReg, Label),
    BranchIfEqual(SrcReg, SrcReg, Label),
    BranchIfNotEqual(SrcReg, SrcReg, Label),
    BranchIfGreaterEqual(SrcReg, SrcReg, Label),
    BranchIfLess(SrcReg, SrcReg, Label),
    Halt, // Used internally when execution finishes.
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(u64);

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

// Inst with its source operands ready for computation.
pub type ReadyInst = Inst<u32, BothReg>;

// Inst with its source operands renamed.
pub type RenamedInst = Inst<ValueOrReg, BothReg>;

// Inst after execution. The source registers are no longer used.
pub type ExecutedInst = Inst<(), BothReg>;

impl FromStr for Inst {
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

        let inst = match op.to_lowercase().as_str() {
            "lb" => Inst::LoadByte(reg_arg(0)?, mem_arg(1)?),
            "lh" => Inst::LoadHalfWord(reg_arg(0)?, mem_arg(1)?),
            "lw" => Inst::LoadWord(reg_arg(0)?, mem_arg(1)?),
            "sb" => Inst::StoreByte(reg_arg(0)?, mem_arg(1)?),
            "sh" => Inst::StoreHalfWord(reg_arg(0)?, mem_arg(1)?),
            "sw" => Inst::StoreWord(reg_arg(0)?, mem_arg(1)?),
            "add" => Inst::Add(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "addi" => Inst::AddImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "andi" => Inst::AndImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "slli" => Inst::ShiftLeftLogicalImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "rem" => Inst::Rem(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "li" => Inst::AddImm(reg_arg(0)?, ArchReg::Zero, imm_arg(1)?),
            "mv" => Inst::AddImm(reg_arg(0)?, reg_arg(1)?, Imm(0)),
            "j" => Inst::Jump(label_arg(0)?),
            "jal" => Inst::JumpAndLink(reg_arg(0)?, label_arg(1)?),
            "beq" => Inst::BranchIfEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bne" => Inst::BranchIfNotEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bge" => Inst::BranchIfGreaterEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "ble" => Inst::BranchIfGreaterEqual(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
            "blt" => Inst::BranchIfLess(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bgt" => Inst::BranchIfLess(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
            "nop" => Inst::nop(),
            "ret" => todo!(),
            _ => return Err(format!("unknown instruction: '{}'", op)),
        };

        Ok(inst)
    }
}

impl Inst {
    pub fn nop() -> Self {
        Inst::AddImm(ArchReg::Zero, ArchReg::Zero, Imm(0))
    }

    pub fn writes_reg(&self, reg: ArchReg) -> bool {
        if reg == ArchReg::Zero {
            return false;
        }

        match self {
            Inst::Add(dst, _, _)
            | Inst::AddImm(dst, _, _)
            | Inst::AndImm(dst, _, _)
            | Inst::ShiftLeftLogicalImm(dst, _, _)
            | Inst::Rem(dst, _, _)
            | Inst::LoadByte(dst, _)
            | Inst::LoadHalfWord(dst, _)
            | Inst::LoadWord(dst, _)
            | Inst::JumpAndLink(dst, _) => *dst == reg,
            Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _)
            | Inst::BranchIfLess(_, _, _)
            | Inst::Jump(_)
            | Inst::StoreByte(_, _)
            | Inst::StoreHalfWord(_, _)
            | Inst::StoreWord(_, _)
            | Inst::Halt => false,
        }
    }
}

impl<SrcReg: Debug + Clone, DstReg: Debug + Clone> Inst<SrcReg, DstReg> {
    pub fn is_branch(&self) -> bool {
        match self {
            Inst::Jump(_)
            | Inst::JumpAndLink(_, _)
            | Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfLess(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _) => true,
            _ => false,
        }
    }

    pub fn is_mem_access(&self) -> bool {
        self.eu_type() == EuType::LoadStore
    }

    pub fn is_load(&self) -> bool {
        match self {
            Inst::LoadByte(_, _) | Inst::LoadHalfWord(_, _) | Inst::LoadWord(_, _) => true,
            _ => false,
        }
    }

    pub fn is_store(&self) -> bool {
        match self {
            Inst::StoreByte(_, _) | Inst::StoreHalfWord(_, _) | Inst::StoreWord(_, _) => true,
            _ => false,
        }
    }

    pub fn eu_type(&self) -> EuType {
        match self {
            Inst::Jump(_)
            | Inst::JumpAndLink(_, _)
            | Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfLess(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _) => EuType::Branch,
            Inst::Add(_, _, _)
            | Inst::AddImm(_, _, _)
            | Inst::AndImm(_, _, _)
            | Inst::ShiftLeftLogicalImm(_, _, _)
            | Inst::Rem(_, _, _) => EuType::ALU,
            Inst::LoadByte(_, _)
            | Inst::LoadHalfWord(_, _)
            | Inst::LoadWord(_, _)
            | Inst::StoreByte(_, _)
            | Inst::StoreHalfWord(_, _)
            | Inst::StoreWord(_, _) => EuType::LoadStore,
            Inst::Halt => EuType::Special,
        }
    }

    pub fn latency(&self) -> u64 {
        match self {
            Inst::Jump(_)
            | Inst::JumpAndLink(_, _)
            | Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfLess(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _) => 1,
            Inst::Add(_, _, _)
            | Inst::AddImm(_, _, _)
            | Inst::AndImm(_, _, _)
            | Inst::ShiftLeftLogicalImm(_, _, _) => 1,
            Inst::Rem(_, _, _) => 3,
            Inst::LoadByte(_, _) | Inst::LoadHalfWord(_, _) | Inst::LoadWord(_, _) => 4,
            Inst::StoreByte(_, _) | Inst::StoreHalfWord(_, _) | Inst::StoreWord(_, _) => 4,
            Inst::Halt => 0,
        }
    }

    #[rustfmt::skip]
    pub fn try_map_regs<OtherSrcReg, OtherDstReg, SrcFn, DstFn>(
        self,
        mut src_fn: SrcFn,
        mut dst_fn: DstFn,
    ) -> Option<Inst<OtherSrcReg, OtherDstReg>>
    where
        OtherSrcReg: Debug + Clone,
        OtherDstReg: Debug + Clone,
        SrcFn: FnMut(SrcReg) -> Option<OtherSrcReg>,
        DstFn: FnMut(DstReg) -> Option<OtherDstReg>,
    {
        Some(match self {
            Inst::Add(dst, src0, src1) => Inst::Add(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::AddImm(dst, src, imm) => Inst::AddImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::AndImm(dst, src, imm) => Inst::AndImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::ShiftLeftLogicalImm(dst, src, imm) => Inst::ShiftLeftLogicalImm(dst_fn(dst)?, src_fn(src)?, imm),
            Inst::Rem(dst, src0, src1) => Inst::Rem(dst_fn(dst)?, src_fn(src0)?, src_fn(src1)?),
            Inst::LoadByte(dst, src) => Inst::LoadByte(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::LoadHalfWord(dst, src) => Inst::LoadHalfWord(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::LoadWord(dst, src) => Inst::LoadWord(dst_fn(dst)?, MemRef { base: src_fn(src.base)?, offset: src.offset }),
            Inst::StoreByte(src, dst) => Inst::StoreByte(src_fn(src)?, MemRef { base: src_fn(dst.base)?, offset: dst.offset }),
            Inst::StoreHalfWord(src, dst) => Inst::StoreHalfWord(src_fn(src)?, MemRef { base: src_fn(dst.base)?, offset: dst.offset }),
            Inst::StoreWord(src, dst) => Inst::StoreWord(src_fn(src)?, MemRef { base: src_fn(dst.base)?, offset: dst.offset }),
            Inst::JumpAndLink(dst, label) => Inst::JumpAndLink(dst_fn(dst)?, label),
            Inst::BranchIfNotEqual(src0, src1, label) => Inst::BranchIfNotEqual(src_fn(src0)?, src_fn(src1)?, label),
            Inst::BranchIfEqual(src0, src1, label) => Inst::BranchIfEqual(src_fn(src0)?, src_fn(src1)?, label),
            Inst::BranchIfGreaterEqual(src0, src1, label)=> Inst::BranchIfGreaterEqual(src_fn(src0)?, src_fn(src1)?, label),
            Inst::BranchIfLess(src0, src1, label)=> Inst::BranchIfLess(src_fn(src0)?, src_fn(src1)?, label),
            Inst::Jump(label) => Inst::Jump(label),
            Inst::Halt => Inst::Halt,
        })
    }

    pub fn map_regs<OtherSrcReg, OtherDstReg, SrcFn, DstFn>(
        self,
        mut src_fn: SrcFn,
        mut dst_fn: DstFn,
    ) -> Inst<OtherSrcReg, OtherDstReg>
    where
        OtherSrcReg: Debug + Clone,
        OtherDstReg: Debug + Clone,
        SrcFn: FnMut(SrcReg) -> OtherSrcReg,
        DstFn: FnMut(DstReg) -> OtherDstReg,
    {
        self.try_map_regs(
            |src_reg| Some(src_fn(src_reg)),
            |dst_reg| Some(dst_fn(dst_reg)),
        )
        .unwrap()
    }

    pub fn map_src_regs<OtherSrcReg, SrcFn>(self, mut src_fn: SrcFn) -> Inst<OtherSrcReg, DstReg>
    where
        OtherSrcReg: Debug + Clone,
        SrcFn: FnMut(SrcReg) -> OtherSrcReg,
    {
        self.map_regs(|src_reg| src_fn(src_reg), |dst_reg| dst_reg)
    }

    pub fn map_dst_regs<OtherDstReg, DstFn>(self, mut dst_fn: DstFn) -> Inst<SrcReg, OtherDstReg>
    where
        OtherDstReg: Debug + Clone,
        DstFn: FnMut(DstReg) -> OtherDstReg,
    {
        self.map_regs(|src_reg| src_reg, |dst_reg| dst_fn(dst_reg))
    }

    pub fn executed(self) -> Inst<(), DstReg> {
        self.map_regs(|_src_reg| (), |dst_reg| dst_reg)
    }
}

impl RenamedInst {
    pub fn get_ready(&self, rf: &RegFile) -> Option<ReadyInst> {
        self.clone().try_map_regs(
            |r| match r {
                ValueOrReg::Value(x) => Some(x),
                ValueOrReg::Reg(phys_reg) => match rf.get_phys(phys_reg) {
                    PrfEntry::Active(x) => Some(x),
                    _ => None,
                },
            },
            |dst_reg| Some(dst_reg),
        )
    }
}

impl FromStr for Imm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let val = if s.starts_with("0x") {
            i64::from_str_radix(&s[2..], 16)
        } else if s.starts_with("-0x") {
            i64::from_str_radix(&s[3..], 16).map(|v| -v)
        } else {
            i64::from_str(s)
        };

        let val = val.map_err(|_| format!("invalid immediate: '{s}'"))?;

        if let Ok(u) = u32::try_from(val) {
            return Ok(Self(u));
        } else if let Ok(s) = i32::try_from(val) {
            assert!(s < 0);
            let abs: u32 = s.abs().try_into().unwrap();
            return Ok(Self(u32::MAX - abs + 1));
        } else {
            return Err(format!("invalid immediate: '{s}'"));
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
