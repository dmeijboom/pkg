use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Script {
    pub body: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Package {
        source: String,
        target: Option<String>,
    },
    Publish {
        target: String
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Package { source, target } => {
                if let Some(target) = target {
                    write!(f, "PACKAGE '{}' AS '{}'", source, target)
                } else {
                    write!(f, "PACKAGE '{}'", source)
                }
            }
            Instruction::Publish { target } => write!(f, "PUBLISH '{}'", target),
        }
    }
}
