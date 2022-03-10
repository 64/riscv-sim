use std::str::FromStr;
use strum::{self, EnumString};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Imm(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MemRef {
    pub base: ArchReg,
    pub offset: Imm,
}

// https://en.wikichip.org/wiki/risc-v/registers
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString)]
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
#[derive(Debug, Clone)]
pub enum Inst {
    LoadByte(ArchReg, MemRef),
    LoadHalfWord(ArchReg, MemRef),
    LoadWord(ArchReg, MemRef),
    StoreByte(ArchReg, MemRef),
    StoreHalfWord(ArchReg, MemRef),
    StoreWord(ArchReg, MemRef),
    Add(ArchReg, ArchReg, ArchReg),
    AddImm(ArchReg, ArchReg, Imm),
    ShiftLeftLogicalImm(ArchReg, ArchReg, Imm),
    JumpAndLink(ArchReg, Imm),
    BranchIfEqual(ArchReg, ArchReg, Label),
    BranchIfNotEqual(ArchReg, ArchReg, Label),
    BranchIfGreaterEqual(ArchReg, ArchReg, Label),
    Halt, // Used internally when execution finishes.
}

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
            "slli" => Inst::ShiftLeftLogicalImm(reg_arg(0)?, reg_arg(1)?, imm_arg(2)?),
            "li" => Inst::AddImm(reg_arg(0)?, ArchReg::Zero, imm_arg(1)?),
            "jal" => Inst::JumpAndLink(reg_arg(0)?, imm_arg(1)?),
            "beq" => Inst::BranchIfEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bne" => Inst::BranchIfNotEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "bge" => Inst::BranchIfGreaterEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "ble" => Inst::BranchIfGreaterEqual(reg_arg(1)?, reg_arg(0)?, label_arg(2)?),
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

    pub fn is_branch(&self) -> bool {
        match self {
            Inst::JumpAndLink(_, _)
            | Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _) => true,
            Inst::LoadByte(_, _)
            | Inst::LoadHalfWord(_, _)
            | Inst::LoadWord(_, _)
            | Inst::StoreByte(_, _)
            | Inst::StoreHalfWord(_, _)
            | Inst::StoreWord(_, _)
            | Inst::Add(_, _, _)
            | Inst::AddImm(_, _, _)
            | Inst::ShiftLeftLogicalImm(_, _, _)
            | Inst::Halt => false,
        }
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

impl Default for ArchReg {
    fn default() -> Self {
        ArchReg::Zero
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
