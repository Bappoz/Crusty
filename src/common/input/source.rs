use crate::common::errors::{report::ToReport, system_error::SystemError};
use memmap2::Mmap;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug)]
pub enum SourceData {
    Mapped(Mmap),
    Memory(String),
}

impl SourceData {
    pub fn as_str(&self) -> &str {
        match self {
            // Assumindo que o codigo do usaurio é UTF8 valido
            SourceData::Mapped(mmap) => std::str::from_utf8(mmap).unwrap_or(""),
            SourceData::Memory(s) => s.as_str(),
        }
    }
}

pub struct SourceFile {
    pub path: PathBuf,
    pub source: SourceData,
    pub(crate) pos: usize,
    line: usize,
    col: usize,
}

impl SourceFile {
    // Lê um arquivo do disco e retorna um SourceFile
    #[warn(clippy::should_implement_trait)]
    pub fn from_path(path: PathBuf) -> Result<Self, Box<dyn ToReport>> {
        let file = File::open(&path).map_err(|e| {
            Box::new(SystemError {
                msg: format!("Could not read file '{}': {}", path.to_string_lossy(), e),
            }) as Box<dyn ToReport>
        })?;

        // Mapeia o arquivo na RAM
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
            Box::new(SystemError {
                msg: format!(
                    "Could not memory map file '{}': {}",
                    path.to_string_lossy(),
                    e
                ),
            }) as Box<dyn ToReport>
        })?;

        Ok(Self {
            path: path.to_owned(),
            source: SourceData::Mapped(mmap),
            pos: 0,
            line: 1,
            col: 1,
        })
    }

    // Cria um SourceFile direto de uma string da memoria
    // Usado para a lib de tests
    pub fn from_string(input: impl Into<String>) -> Self {
        Self {
            path: PathBuf::from("<string>"),
            source: SourceData::Memory(input.into()),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.source.as_str()[self.pos..].chars().next()
    }

    // Olha o char APÓS o próximo sem avançar.
    pub fn peek_ahead(&self) -> Option<char> {
        let mut chars = self.source.as_str()[self.pos..].chars();
        chars.next();
        chars.next()
    }

    pub fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    // Avança somente se o próximo char satisfizer o predicado.
    // Retorna true se avançou.
    pub fn advance_if(&mut self, f: impl Fn(char) -> bool) -> bool {
        match self.peek() {
            Some(c) if f(c) => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    // Checa se acabou o contexto
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.source.as_str().len()
    }

    // Getters
    pub fn current_pos(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
    }

    // Setters
    pub fn set_line(&mut self, new_line: usize) {
        self.line = new_line
    }

    pub fn set_col(&mut self, new_col: usize) {
        self.col = new_col
    }
}
