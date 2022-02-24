use std::str::FromStr;
use std::collections::HashMap;
use crate::inst::{Inst, Label};

#[derive(Debug)]
pub struct Program {
    pub insts: Vec<Inst>,
    pub labels: HashMap<Label, usize>, // indices into the insts array
}

impl FromStr for Program {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut insts = Vec::new();
        let mut labels = HashMap::new();

        for (i, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.ends_with(':') {
                match Label::from_str(&line[0..line.len() - 1]) {
                    Ok(label) => labels.insert(label, insts.len()),
                    Err(e) => return Err(format!("error parsing label on line {i}: {e}")),
                };
            } else {
                match Inst::from_str(line) {
                    Ok(inst) => insts.push(inst),
                    Err(e) => return Err(format!("error parsing instruction on line {i}: {e}")),
                }
            }
        }

        Ok(Program { insts, labels })
    }
}
