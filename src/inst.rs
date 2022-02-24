use num_enum::TryFromPrimitive;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub struct Imm(pub u32);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Label(pub String);

#[derive(Debug, PartialEq, Eq)]
pub struct MemRef {
    base: Imm,
    offset: ArchReg,
}

#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum ArchReg {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Zero,
    Stack,
    Return,
}

#[derive(Debug)]
pub enum Inst {
    LoadImm(ArchReg, Imm),
    LoadByte(ArchReg, MemRef),
    LoadHalfWord(ArchReg, MemRef),
    LoadWord(ArchReg, MemRef),
    StoreByte(MemRef, ArchReg),
    StoreHalfWord(MemRef, ArchReg),
    StoreWord(MemRef, ArchReg),
    Add(ArchReg, ArchReg, ArchReg),
    Mul(ArchReg, ArchReg, ArchReg),
    Not(ArchReg),
    Jump(Label),
    JumpIfEqual(ArchReg, ArchReg, Label),
    JumpIfNotEqual(ArchReg, ArchReg, Label),
    Nop,
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
        let reg_arg = |n: usize| -> Result<ArchReg, String> { ArchReg::from_str(nth_arg(n)?) };
        let mem_arg = |n: usize| -> Result<MemRef, String> { MemRef::from_str(nth_arg(n)?) };
        let imm_arg = |n: usize| -> Result<Imm, String> { Imm::from_str(nth_arg(n)?) };
        let label_arg = |n: usize| -> Result<Label, String> { Label::from_str(nth_arg(n)?) };

        let inst = match op.to_lowercase().as_str() {
            "nop" => Inst::Nop,
            "loadi" => Inst::LoadImm(reg_arg(0)?, imm_arg(1)?),
            "loadb" => Inst::LoadByte(reg_arg(0)?, mem_arg(1)?),
            "loadh" => Inst::LoadHalfWord(reg_arg(0)?, mem_arg(1)?),
            "loadw" => Inst::LoadWord(reg_arg(0)?, mem_arg(1)?),
            "storeb" => Inst::StoreByte(mem_arg(0)?, reg_arg(1)?),
            "storeh" => Inst::StoreHalfWord(mem_arg(0)?, reg_arg(1)?),
            "storew" => Inst::StoreWord(mem_arg(0)?, reg_arg(1)?),
            "add" => Inst::Add(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "mul" => Inst::Mul(reg_arg(0)?, reg_arg(1)?, reg_arg(2)?),
            "not" => Inst::Not(reg_arg(0)?),
            "jmp" => Inst::Jump(label_arg(0)?),
            "jeq" => Inst::JumpIfEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            "jne" => Inst::JumpIfNotEqual(reg_arg(0)?, reg_arg(1)?, label_arg(2)?),
            _ => return Err(format!("unknown instruction: '{}'", op)),
        };

        Ok(inst)
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
            return Ok(Self(u32::MAX - abs)); 
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
        let inner = s
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .map(|s| s.trim())
            .ok_or_else(|| format!("invalid memory reference (no []): '{s}'"))?;

        if let Ok(reg) = inner.parse::<ArchReg>() {
            return Ok(MemRef {
                base: Imm(0),
                offset: reg,
            });
        } else if let Ok(imm) = inner.parse::<Imm>() {
            return Ok(MemRef {
                base: imm,
                offset: ArchReg::Zero,
            });
        }

        if inner.matches(&['+', '-']).count() > 1 {
            return Err(format!("invalid memory reference (too many +-): '{s}'"));
        }

        let (fst, snd) = inner
            .split_once(&['+', '-'])
            .ok_or_else(|| format!("invalid memory reference (no +-): '{s}'"))?;
        let (fst, snd) = (fst.trim(), snd.trim());

        // Handle [reg + imm], [reg - imm], [imm + reg]
        let plus_used = inner.find('+').is_some();
        if plus_used {
            if let (Ok(reg), Ok(imm)) = (ArchReg::from_str(fst), Imm::from_str(snd)) {
                return Ok(Self { base: imm, offset: reg });
            } else if let (Ok(imm), Ok(reg)) = (Imm::from_str(fst), ArchReg::from_str(snd)) {
                return Ok(Self { base: imm, offset: reg });
            }
        } else {
            if let (Ok(reg), Ok(imm)) = (ArchReg::from_str(fst), Imm::from_str(snd)) {
                return Ok(Self { base: Imm(imm.0.wrapping_neg()), offset: reg });
            }
        }

        return Err(format!("invalid memory reference (not matched): '{s}'"));
    }
}

impl FromStr for ArchReg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "zero" => ArchReg::Zero,
            "sp" => ArchReg::Stack,
            "ra" => ArchReg::Return,
            _ if s.starts_with('r') => match s[1..].parse::<u8>().map(|i| ArchReg::try_from(i)) {
                Ok(Ok(reg)) => reg,
                Ok(Err(e)) => return Err(e.to_string()),
                Err(e) => return Err(e.to_string()),
            },
            _ => return Err(format!("unknown register: '{s}'")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reg() {
        assert_eq!(ArchReg::from_str("zero"), Ok(ArchReg::Zero));
        assert_eq!(ArchReg::from_str("sp"), Ok(ArchReg::Stack));
        assert_eq!(ArchReg::from_str("ra"), Ok(ArchReg::Return));
        assert_eq!(ArchReg::from_str("r0"), Ok(ArchReg::R0));
        assert_eq!(ArchReg::from_str("r1"), Ok(ArchReg::R1));
        assert_eq!(ArchReg::from_str("r11"), Ok(ArchReg::R11));
        assert_eq!(ArchReg::from_str("r15"), Ok(ArchReg::R15));
        assert!(ArchReg::from_str("0").is_err());
        assert!(ArchReg::from_str("r-0").is_err());
        assert!(ArchReg::from_str("r-1").is_err());
        assert!(ArchReg::from_str("r50").is_err());
    }

    #[test]
    fn test_memref() {
        assert_eq!(MemRef::from_str("[r1]"), Ok(MemRef { base: Imm(0), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 + 0]"), Ok(MemRef { base: Imm(0), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[0 + r1]"), Ok(MemRef { base: Imm(0), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1+5]"), Ok(MemRef { base: Imm(5), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 - 0]"), Ok(MemRef { base: Imm(0), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[zero - 0]"), Ok(MemRef { base: Imm(0), offset: ArchReg::Zero }));
        assert_eq!(MemRef::from_str("[r1 + 1]"), Ok(MemRef { base: Imm(1), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 + 0x123]"), Ok(MemRef { base: Imm(0x123), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 - 1]"), Ok(MemRef { base: Imm(u32::MAX), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 -1]"), Ok(MemRef { base: Imm(u32::MAX), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 - 2]"), Ok(MemRef { base: Imm(u32::MAX - 1), offset: ArchReg::R1 }));
        assert_eq!(MemRef::from_str("[r1 - 0x123]"), Ok(MemRef { base: Imm(u32::MAX - 0x123 + 1), offset: ArchReg::R1 }));

        assert!(MemRef::from_str("[-r1 + 0]").is_err());
        assert!(MemRef::from_str("[r1 + 0 + 0]").is_err());
        assert!(MemRef::from_str("[0 + r1 + 0]").is_err());
        assert!(MemRef::from_str("[0 - r1]").is_err());
        assert!(MemRef::from_str("[0 - +r1]").is_err());
        assert!(MemRef::from_str("[r1 - +0]").is_err());
        assert!(MemRef::from_str("[r1 + -0]").is_err());
    }

    #[test]
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
