use crate::inst::Inst;

pub fn read_after_write(a: &Option<Inst>, b: &Option<Inst>) -> bool {
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };

    match *a {
        Inst::StoreByte(src, dst) | Inst::StoreHalfWord(src, dst) | Inst::StoreWord(src, dst) => {
            b.writes_reg(src) || b.writes_reg(dst.base)
        }
        Inst::LoadByte(_, src) | Inst::LoadHalfWord(_, src) | Inst::LoadWord(_, src) => {
            b.writes_reg(src.base)
        }
        Inst::BranchIfNotEqual(src0, src1, _)
        | Inst::BranchIfEqual(src0, src1, _)
        | Inst::BranchIfGreaterEqual(src0, src1, _)
        | Inst::Add(_, src0, src1) => b.writes_reg(src0) || b.writes_reg(src1),
        Inst::ShiftLeftLogicalImm(_, src, _) | Inst::AddImm(_, src, _) => b.writes_reg(src),
        Inst::JumpAndLink(_, _) | Inst::Halt => false,
    }
}
