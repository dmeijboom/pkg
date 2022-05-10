use anyhow::{anyhow, Result};

use crate::pkgscript::ast::Script;
use crate::pkgscript::Instruction;

pub struct Parser<'s> {
    pos: usize,
    source: &'s str,
}

impl<'s> Parser<'s> {
    fn eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    pub fn parse(source: &str) -> Result<Script> {
        let mut script = Script { body: vec![] };
        let mut parser = Parser {
            pos: 0,
            source: source.trim(),
        };

        while !parser.eof() {
            parser.skip_whitespaces();

            match parser.peek_name()? {
                "PACKAGE" => script.body.push(parser.parse_package()?),
                "PUBLISH" => script.body.push(parser.parse_publish()?),
                name => return Err(anyhow!("invalid instruction: {}", name)),
            }
        }

        Ok(script)
    }

    fn skip_whitespaces(&mut self) {
        while !self.eof() {
            if !self.source[self.pos..self.pos + 1]
                .chars()
                .next()
                .unwrap()
                .is_whitespace()
            {
                return;
            }

            self.pos += 1;
        }
    }

    fn peek_name(&mut self) -> Result<&str> {
        let end = self.source[self.pos..]
            .find(" ")
            .ok_or_else(|| anyhow!("unknown instruction"))?;

        Ok(&self.source[self.pos..self.pos + end])
    }

    fn parse_path(&mut self) -> Result<String> {
        self.skip_whitespaces();

        let mut path = String::new();
        let mut chars = self.source[self.pos..].chars();

        while let Some(c) = chars.next() {
            match c {
                '\n' => break,
                c if c.is_whitespace() => break,
                _ => path.push(c),
            }
        }

        self.pos += path.len();

        Ok(path)
    }

    fn parse_publish(&mut self) -> Result<Instruction> {
        self.pos += "PUBLISH ".len();
        self.skip_whitespaces();

        let target = self.parse_path()?;

        Ok(Instruction::Publish { target })
    }

    fn parse_package(&mut self) -> Result<Instruction> {
        self.pos += "PACKAGE ".len();
        self.skip_whitespaces();

        let source = self.parse_path()?;

        self.skip_whitespaces();

        let mut target = None;

        if matches!(self.peek_name(), Ok("AS")) {
            self.pos += "AS ".len();

            target = Some(self.parse_path()?);
        }

        Ok(Instruction::Package { source, target })
    }
}
