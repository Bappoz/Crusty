#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub end_line: usize,
    pub column_start: usize,
    pub column_end: usize,
}

pub struct Source {
    pub filename: String,
    pub lines: Vec<String>,
}

impl Source {
    /// Cria um `Source` a partir do nome do arquivo e seu conteúdo completo, dividindo em linhas.
    pub fn new(filename: &str, content: &str) -> Self {
        Self {
            filename: filename.to_string(),
            lines: content.lines().map(|line| line.to_string()).collect(),
        }
    }

    /// Retorna a linha de número `line` (1-indexed), ou `None` se o índice estiver fora do intervalo.
    pub fn get_lines(&self, line: usize) -> Option<&str> {
        if line == 0 {
            return None;
        }
        self.lines.get(line - 1).map(|s| s.as_str())
    }
}

// Setinhas no error
#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,
}
