use crate::inst::{Inst, Label, LabeledInst, Pc};
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone)]
pub struct Program {
    pub insts: Vec<Inst>,
    pub labels: HashMap<Label, Pc>,
}

impl FromStr for Program {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut insts = Vec::default();
        let mut labels = HashMap::new();

        for (i, line) in s.lines().enumerate() {
            // Strip comments and empty lines
            let line = line.trim();
            let line = &line[..line.find(";").unwrap_or(line.len())];
            if line.is_empty() {
                continue;
            }

            // Line numbers start at 1
            let i = i + 1;

            if line.ends_with(':') {
                match Label::from_str(&line[0..line.len() - 1]) {
                    Ok(label) => labels.insert(label, insts.len().try_into().unwrap()),
                    Err(e) => return Err(format!("error parsing label on line {i}: {e}")),
                };
            } else {
                match LabeledInst::from_str(line) {
                    Ok(inst) => insts.push(inst),
                    Err(e) => {
                        return Err(format!(
                            "error parsing instruction '{line}' on line {i}: {e}"
                        ))
                    }
                }
            }
        }

        // Do another pass to fixup the labels.
        let insts = insts
            .into_iter()
            .map(|inst| inst.map_jumps(|tgt| labels[&tgt]))
            .collect();

        Ok(Program { insts, labels })
    }
}

impl Program {
    pub fn fetch(&self, ip: u32) -> Option<&Inst> {
        self.insts.get(usize::try_from(ip).unwrap())
    }
}
