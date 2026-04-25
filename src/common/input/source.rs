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
    /// Retorna o conteúdo do source como `&str`, assumindo UTF-8 válido.
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
    /// Lê um arquivo do disco via memory-map e retorna um `SourceFile` pronto para leitura.
    #[allow(clippy::should_implement_trait)]
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

    /// Cria um `SourceFile` diretamente de uma string em memória; usado principalmente em testes.
    pub fn from_string(input: impl Into<String>) -> Self {
        Self {
            path: PathBuf::from("<string>"),
            source: SourceData::Memory(input.into()),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Retorna o próximo caractere sem avançar a posição atual no source.
    pub fn peek(&self) -> Option<char> {
        self.source.as_str()[self.pos..].chars().next()
    }

    /// Retorna o caractere dois passos à frente sem avançar (lookahead de 2).
    pub fn peek_ahead(&self) -> Option<char> {
        let mut chars = self.source.as_str()[self.pos..].chars();
        chars.next();
        chars.next()
    }

    /// Consome e retorna o próximo caractere, atualizando linha e coluna.
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

    /// Avança somente se o próximo char satisfizer o predicado; retorna `true` se avançou.
    pub fn advance_if(&mut self, f: impl Fn(char) -> bool) -> bool {
        match self.peek() {
            Some(c) if f(c) => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    /// Retorna `true` se toda a entrada já foi consumida.
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.source.as_str().len()
    }

    /// Retorna a posição atual como tupla `(linha, coluna)`.
    pub fn current_pos(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    /// Retorna o número da linha atual (1-indexed).
    pub fn line(&self) -> usize {
        self.line
    }

    /// Retorna o número da coluna atual (1-indexed).
    pub fn col(&self) -> usize {
        self.col
    }

    /// Define o número da linha atual; usado para sincronização após diretivas de pré-processador.
    pub fn set_line(&mut self, new_line: usize) {
        self.line = new_line
    }

    /// Define o número da coluna atual; usado para sincronização após diretivas de pré-processador.
    pub fn set_col(&mut self, new_col: usize) {
        self.col = new_col
    }
}
