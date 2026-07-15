use std::iter::repeat_n;

#[derive(Copy, Clone)]
pub struct IndentedLine<'a> {
    pub indent: usize,
    pub text: &'a str,
}

impl<'a> From<&'a str> for IndentedLine<'a> {
    fn from(mut text: &'a str) -> Self {
        let mut indent: usize = 0;
        while !text.is_empty() {
            let space_size = match text.as_bytes()[0] {
                b' ' => 1,
                b'\t' => 4,
                _ => break,
            };
            indent += space_size;
            text = &text[1..];
        }
        Self { indent, text }
    }
}

pub trait IndentedLineIterExt<'a>: Iterator<Item = IndentedLine<'a>> {
    fn common_indent(&mut self) -> usize;
    fn indented(&mut self, indent: isize) -> impl Iterator<Item = IndentedLine<'a>>;
}

impl<'a, T> IndentedLineIterExt<'a> for T
where
    T: Iterator<Item = IndentedLine<'a>>,
{
    fn common_indent(&mut self) -> usize {
        self.map(|line| line.indent).min().unwrap_or(0)
    }

    fn indented(&mut self, indent: isize) -> impl Iterator<Item = IndentedLine<'a>> {
        self.map(move |line| IndentedLine {
            indent: line.indent.saturating_add_signed(indent),
            ..line
        })
    }
}

impl<'a> Extend<IndentedLine<'a>> for String {
    fn extend<T: IntoIterator<Item=IndentedLine<'a>>>(&mut self, iter: T) {
        for line in iter.into_iter() {
            self.extend(repeat_n(' ', line.indent));
            self.push_str(line.text);
            self.push('\n');
        }
    }
}

impl<'a> FromIterator<IndentedLine<'a>> for String {
    fn from_iter<T: IntoIterator<Item=IndentedLine<'a>>>(iter: T) -> Self {
        let mut string = String::new();
        string.extend(iter);
        string
    }
}
