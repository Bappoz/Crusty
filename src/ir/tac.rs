#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(pub u32);

#[derive(Debug, Clone)]
pub struct TempGen {
    next: u32,
}

impl TempGen {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn fresh(&mut self) -> TempId {
        let temp = TempId(self.next);
        self.next += 1;
        temp
    }
}

impl Default for TempGen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(pub u32);

#[derive(Debug, Clone)]
pub struct LabelGen {
    next: u32,
}

impl LabelGen {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn fresh(&mut self) -> LabelId {
        let label = LabelId(self.next);
        self.next += 1;
        label
    }
}

impl Default for LabelGen {
    fn default() -> Self {
        Self::new()
    }
}
