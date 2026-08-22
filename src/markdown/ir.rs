#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Para(Vec<Span>),
    Quote {
        wide: bool,
        blocks: Vec<Block>,
    },
    Code {
        lang: String,
        body: String,
    },
    Heading(u8, Vec<Span>),
    Subtext(Vec<Span>),
    List {
        ordered: bool,
        items: Vec<Vec<Span>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    Text(String),
    Bold(Vec<Span>),
    Italic(Vec<Span>),
    Underline(Vec<Span>),
    Strike(Vec<Span>),
    Spoiler(Vec<Span>),
    Code(String),
    Link {
        url: String,
        label: Vec<Span>,
    },
    Autolink(String),
    Emoji {
        id: String,
        name: String,
        animated: bool,
    },
    Mention(MentionKind),
    Timestamp {
        unix: i64,
        style: char,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionKind {
    User(String),
    Channel(String),
    Role(String),
    Everyone,
    Here,
}
