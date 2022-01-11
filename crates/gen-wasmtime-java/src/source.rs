use std::{fmt, iter};

const INDENT: usize = 4;

#[derive(Default)]
pub struct Source {
    s: String,
    indent: usize,
}

impl Source {
    pub fn push_lines(&mut self, src: impl AsRef<str>) {
        let lines = src.as_ref().lines().collect::<Vec<_>>();
        for line in lines {
            let line = line.trim();
            self.push_indent();
            self.s.push_str(line);
            self.newline();
        }
    }

    fn push_indent(&mut self) {
        self.s.extend(iter::repeat(" ").take(self.indent));
    }

    pub fn indent(&mut self) {
        self.indent += INDENT;
    }

    pub fn outdent(&mut self) {
        self.indent = self.indent.saturating_sub(INDENT);
    }

    fn newline(&mut self) {
        self.s.push_str("\n");
    }

    pub fn as_str(&self) -> &str {
        &self.s
    }
}

impl std::ops::Deref for Source {
    type Target = str;
    fn deref(&self) -> &str {
        &self.s
    }
}

impl From<Source> for String {
    fn from(s: Source) -> String {
        s.s
    }
}
