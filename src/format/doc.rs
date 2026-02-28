#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doc {
    Text(String),
    Space,
    Line,     // hard line break
    SoftLine, // space or break based on layout
    Indent(Box<Doc>),
    Group(Vec<Doc>),
    Concat(Vec<Doc>),
}
