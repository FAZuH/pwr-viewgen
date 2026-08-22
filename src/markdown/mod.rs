pub mod ir;
pub mod timestamp;

pub use ir::Block;
pub use ir::MentionKind;
pub use ir::Span;

const MAX_INLINE_DEPTH: usize = 16;

pub fn parse(input: &str) -> Vec<Block> {
    let lines: Vec<&str> = input.lines().collect();
    parse_blocks(&lines)
}

fn parse_blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(lang) = line.strip_prefix("```") {
            let mut body_lines = Vec::new();
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim() != "```" {
                body_lines.push(lines[j]);
                j += 1;
            }
            if j < lines.len() {
                j += 1;
            }
            blocks.push(Block::Code {
                lang: lang.trim().to_owned(),
                body: body_lines.join("\n"),
            });
            i = j;
            continue;
        }
        if let Some(rest) = line.strip_prefix("-# ") {
            blocks.push(Block::Subtext(parse_inline(rest, 0)));
            i += 1;
            continue;
        }
        if let Some((level, rest)) = heading_level(line) {
            blocks.push(Block::Heading(level, parse_inline(rest, 0)));
            i += 1;
            continue;
        }
        if line == ">>>" || line.starts_with(">>> ") {
            let mut inner: Vec<&str> = Vec::new();
            if let Some(rest) = line.strip_prefix(">>> ") {
                inner.push(rest);
            }
            inner.extend_from_slice(&lines[i + 1..]);
            blocks.push(Block::Quote {
                wide: true,
                blocks: parse_blocks(&inner),
            });
            break;
        }
        if line == ">" || line.starts_with("> ") {
            let mut inner: Vec<&str> = Vec::new();
            let mut j = i;
            while j < lines.len() {
                let l = lines[j];
                if l == ">" {
                    inner.push("");
                } else if let Some(rest) = l.strip_prefix("> ") {
                    inner.push(rest);
                } else {
                    break;
                }
                j += 1;
            }
            blocks.push(Block::Quote {
                wide: false,
                blocks: parse_blocks(&inner),
            });
            i = j;
            continue;
        }
        if list_marker(line).is_some() {
            let mut items = Vec::new();
            let ordered = matches!(list_marker(line), Some((true, _)));
            let mut j = i;
            while j < lines.len() {
                match list_marker(lines[j]) {
                    Some((same_kind, content)) if same_kind == ordered => {
                        items.push(parse_inline(content, 0));
                        j += 1;
                    }
                    _ => break,
                }
            }
            blocks.push(Block::List { ordered, items });
            i = j;
            continue;
        }

        let mut paragraph: Vec<&str> = vec![line];
        let mut j = i + 1;
        while j < lines.len() && !lines[j].trim().is_empty() && !is_block_start(lines[j]) {
            paragraph.push(lines[j]);
            j += 1;
        }
        blocks.push(Block::Para(parse_inline(&paragraph.join("\n"), 0)));
        i = j;
    }
    blocks
}

fn heading_level(line: &str) -> Option<(u8, &str)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if (1..=3).contains(&hashes) {
        let rest = &line[hashes..];
        return rest.strip_prefix(' ').map(|text| (hashes as u8, text));
    }
    None
}

fn list_marker(line: &str) -> Option<(bool, &str)> {
    if let Some(rest) = line.strip_prefix("- ") {
        return Some((false, rest));
    }
    let digits_end = line
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(line.len());
    if digits_end > 0 && line[digits_end..].starts_with(". ") {
        return Some((true, &line[digits_end + 2..]));
    }
    None
}

fn is_block_start(line: &str) -> bool {
    line.starts_with("```")
        || line == ">"
        || line.starts_with("> ")
        || line == ">>>"
        || line.starts_with(">>> ")
        || line.starts_with("-# ")
        || heading_level(line).is_some()
        || list_marker(line).is_some()
}

fn parse_inline(text: &str, depth: usize) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut literal = String::new();
    let mut pos = 0usize;

    while pos < text.len() {
        let rest = &text[pos..];
        let prev = prev_char_before(text, pos);
        let matched = try_rules(rest, prev, depth).map(|(span, len)| {
            flush_literal(&mut spans, &mut literal);
            spans.push(span);
            len
        });
        if let Some(len) = matched {
            pos += len;
        } else if let Some(escaped) = try_escape(rest) {
            literal.push(escaped);
            pos += 1 + escaped.len_utf8();
        } else {
            let ch = rest.chars().next().unwrap_or('\u{fffd}');
            literal.push(ch);
            pos += ch.len_utf8();
        }
    }
    flush_literal(&mut spans, &mut literal);
    spans
}

fn flush_literal(spans: &mut Vec<Span>, literal: &mut String) {
    if !literal.is_empty() {
        spans.push(Span::Text(std::mem::take(literal)));
    }
}

fn try_rules(rest: &str, prev: Option<char>, depth: usize) -> Option<(Span, usize)> {
    rule_emoji(rest)
        .or_else(|| rule_mention(rest))
        .or_else(|| rule_timestamp(rest))
        .or_else(|| paired(rest, "||", depth, Span::Spoiler))
        .or_else(|| paired(rest, "**", depth, Span::Bold))
        .or_else(|| paired(rest, "__", depth, Span::Underline))
        .or_else(|| rule_italic(rest, prev, depth))
        .or_else(|| paired(rest, "~~", depth, Span::Strike))
        .or_else(|| rule_code(rest))
        .or_else(|| rule_link(rest, depth))
        .or_else(|| rule_autolink(rest))
}

fn try_escape(rest: &str) -> Option<char> {
    let mut chars = rest.chars();
    if chars.next()? != '\\' {
        return None;
    }
    match chars.next() {
        Some(c) if c.is_ascii_punctuation() => Some(c),
        _ => None,
    }
}

fn paired(
    rest: &str,
    marker: &str,
    depth: usize,
    build: fn(Vec<Span>) -> Span,
) -> Option<(Span, usize)> {
    if depth >= MAX_INLINE_DEPTH {
        return None;
    }
    let inner_full = rest.strip_prefix(marker)?;
    let close = inner_full.find(marker)?;
    if close == 0 {
        return None;
    }
    let inner = &inner_full[..close];
    Some((
        build(parse_inline(inner, depth + 1)),
        marker.len() * 2 + inner.len(),
    ))
}

fn rule_italic(rest: &str, prev: Option<char>, depth: usize) -> Option<(Span, usize)> {
    let marker = if rest.starts_with('*') {
        '*'
    } else if rest.starts_with('_') {
        '_'
    } else {
        return None;
    };
    if marker == '_' && prev.is_some_and(is_word_char) {
        return None;
    }
    if depth >= MAX_INLINE_DEPTH {
        return None;
    }
    let inner_full = &rest[1..];
    let close = inner_full.find(marker)?;
    if close == 0 {
        return None;
    }
    let inner = &inner_full[..close];
    if marker == '_'
        && inner_full[close + 1..]
            .chars()
            .next()
            .is_some_and(is_word_char)
    {
        return None;
    }
    Some((
        Span::Italic(parse_inline(inner, depth + 1)),
        2 + inner.len(),
    ))
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn rule_code(rest: &str) -> Option<(Span, usize)> {
    let inner_full = rest.strip_prefix('`')?;
    let close = inner_full.find('`')?;
    if close == 0 {
        return None;
    }
    Some((Span::Code(inner_full[..close].to_owned()), close + 2))
}

fn rule_link(rest: &str, depth: usize) -> Option<(Span, usize)> {
    if depth >= MAX_INLINE_DEPTH {
        return None;
    }
    let label_area = rest.strip_prefix('[')?;
    let close_label = label_area.find("](")?;
    let url_area = &label_area[close_label + 2..];
    let close_url = url_area.find(')')?;
    let url = &url_area[..close_url];
    if url.is_empty() {
        return None;
    }
    Some((
        Span::Link {
            url: url.to_owned(),
            label: parse_inline(&label_area[..close_label], depth + 1),
        },
        1 + close_label + 2 + close_url + 1,
    ))
}

fn rule_autolink(rest: &str) -> Option<(Span, usize)> {
    const HTTPS: &[u8] = b"https://";
    const HTTP: &[u8] = b"http://";
    let scheme_len = if rest.len() >= HTTPS.len()
        && rest.as_bytes()[..HTTPS.len()].eq_ignore_ascii_case(HTTPS)
    {
        HTTPS.len()
    } else if rest.len() >= HTTP.len() && rest.as_bytes()[..HTTP.len()].eq_ignore_ascii_case(HTTP) {
        HTTP.len()
    } else {
        return None;
    };
    let body = &rest[scheme_len..];
    let end_ws = body.find(char::is_whitespace).unwrap_or(body.len());
    let end_lt = body.find('<').unwrap_or(body.len());
    let mut url = &rest[..scheme_len + end_ws.min(end_lt)];
    url = trim_url_punctuation(url);
    if url.len() <= scheme_len {
        return None;
    }
    Some((Span::Autolink(url.to_owned()), url.len()))
}

fn trim_url_punctuation(mut url: &str) -> &str {
    loop {
        let Some(last) = url.chars().next_back() else {
            return url;
        };
        if matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"') {
            url = &url[..url.len() - last.len_utf8()];
        } else if last == ')' {
            let opens = url.bytes().filter(|&b| b == b'(').count();
            let closes = url.bytes().filter(|&b| b == b')').count();
            if closes > opens {
                url = &url[..url.len() - 1];
            } else {
                return url;
            }
        } else {
            return url;
        }
    }
}

fn rule_emoji(rest: &str) -> Option<(Span, usize)> {
    let (body, animated, prefix_len) = match rest.strip_prefix("<a:") {
        Some(b) => (b, true, 3),
        None => (rest.strip_prefix("<:")?, false, 2),
    };
    let close = body.find('>')?;
    let spec = &body[..close];
    let (name, id) = spec.split_once(':')?;
    if name.is_empty()
        || id.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || !id.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((
        Span::Emoji {
            id: id.to_owned(),
            name: name.to_owned(),
            animated,
        },
        prefix_len + close + 1,
    ))
}

fn rule_mention(rest: &str) -> Option<(Span, usize)> {
    for (prefix, kind) in [("<@&", 4), ("<@", 3), ("<#", 3)] {
        if let Some(body) = rest.strip_prefix(prefix) {
            let close = body.find('>')?;
            let id = &body[..close];
            if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) {
                let span = match kind {
                    4 => MentionKind::Role(id.to_owned()),
                    3 => {
                        if rest.starts_with("<@") {
                            MentionKind::User(id.to_owned())
                        } else {
                            MentionKind::Channel(id.to_owned())
                        }
                    }
                    _ => return None,
                };
                return Some((Span::Mention(span), prefix.len() + close + 1));
            }
            return None;
        }
    }
    if let Some(after) = rest.strip_prefix("@everyone") {
        if after.chars().next().is_none_or(|c| !is_word_char(c)) {
            return Some((Span::Mention(MentionKind::Everyone), 9));
        }
    }
    if let Some(after) = rest.strip_prefix("@here") {
        if after.chars().next().is_none_or(|c| !is_word_char(c)) {
            return Some((Span::Mention(MentionKind::Here), 5));
        }
    }
    None
}

fn rule_timestamp(rest: &str) -> Option<(Span, usize)> {
    let inner = rest.strip_prefix("<t:")?;
    let close = inner.find('>')?;
    let spec = &inner[..close];
    let (digits_raw, style) = match spec.split_once(':') {
        Some((digits, style)) => {
            let mut chars = style.chars();
            let c = chars.next()?;
            if chars.next().is_some() || !timestamp::is_valid_style(c) {
                return None;
            }
            (digits, c)
        }
        None => (spec, 'f'),
    };
    let unsigned_part = digits_raw.strip_prefix('-').unwrap_or(digits_raw);
    if unsigned_part.is_empty() || !unsigned_part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let unix: i64 = digits_raw.parse().ok()?;
    Some((Span::Timestamp { unix, style }, 3 + close + 1))
}

fn prev_char_before(text: &str, pos: usize) -> Option<char> {
    text[..pos].chars().next_back()
}
#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Span {
        Span::Text(s.to_owned())
    }

    fn plain(parts: &[&str]) -> Vec<Span> {
        parts.iter().map(|p| t(p)).collect()
    }

    // ---------- inline rules, table-driven ----------

    #[test]
    fn inline_table() {
        let cases: Vec<(&str, Vec<Span>)> = vec![
            ("hello world", plain(&["hello world"])),
            ("**hi**", vec![Span::Bold(plain(&["hi"]))]),
            ("**unclosed", plain(&["**unclosed"])),
            ("****", plain(&["****"])),
            ("__under__", vec![Span::Underline(plain(&["under"]))]),
            ("*it*", vec![Span::Italic(plain(&["it"]))]),
            ("_it_", vec![Span::Italic(plain(&["it"]))]),
            ("snake_case_name", plain(&["snake_case_name"])),
            ("a*b*c", vec![t("a"), Span::Italic(plain(&["b"])), t("c")]),
            ("~~gone~~", vec![Span::Strike(plain(&["gone"]))]),
            ("~~open", plain(&["~~open"])),
            ("||secret||", vec![Span::Spoiler(plain(&["secret"]))]),
            ("||oops", plain(&["||oops"])),
            ("||||", plain(&["||||"])),
            ("`**raw**`", vec![Span::Code("**raw**".to_owned())]),
            ("``", plain(&["``"])),
            (
                "**bold _it_**",
                vec![Span::Bold(vec![t("bold "), Span::Italic(plain(&["it"]))])],
            ),
            (
                "||spoiler **b**||",
                vec![Span::Spoiler(vec![
                    t("spoiler "),
                    Span::Bold(plain(&["b"])),
                ])],
            ),
            (r"\*not italic\*", plain(&["*not italic*"])),
            (r"a\b", plain(&[r"a\b"])),
            (
                "<:tada:123456789>",
                vec![Span::Emoji {
                    id: "123456789".to_owned(),
                    name: "tada".to_owned(),
                    animated: false,
                }],
            ),
            (
                "<a:dance:42>",
                vec![Span::Emoji {
                    id: "42".to_owned(),
                    name: "dance".to_owned(),
                    animated: true,
                }],
            ),
            ("<:noid>", plain(&["<:noid>"])),
            ("<:x:abc>", plain(&["<:x:abc>"])),
            (
                "<@123>",
                vec![Span::Mention(MentionKind::User("123".into()))],
            ),
            (
                "<@&777>",
                vec![Span::Mention(MentionKind::Role("777".into()))],
            ),
            (
                "<#555>",
                vec![Span::Mention(MentionKind::Channel("555".into()))],
            ),
            ("<@>", plain(&["<@>"])),
            (
                "ping @everyone!",
                vec![t("ping "), Span::Mention(MentionKind::Everyone), t("!")],
            ),
            (
                "hey @here.",
                vec![t("hey "), Span::Mention(MentionKind::Here), t(".")],
            ),
            ("@everyonez stays", plain(&["@everyonez stays"])),
            (
                "<t:0:f>",
                vec![Span::Timestamp {
                    unix: 0,
                    style: 'f',
                }],
            ),
            (
                "<t:1709164800>",
                vec![Span::Timestamp {
                    unix: 1_709_164_800,
                    style: 'f',
                }],
            ),
            (
                "<t:-5:t>",
                vec![Span::Timestamp {
                    unix: -5,
                    style: 't',
                }],
            ),
            ("<t:1:x>", plain(&["<t:1:x>"])),
            ("<t:zz>", plain(&["<t:zz>"])),
            (
                "[click](https://x.test/a)",
                vec![Span::Link {
                    url: "https://x.test/a".to_owned(),
                    label: plain(&["click"]),
                }],
            ),
            (
                "[**b**](u)",
                vec![Span::Link {
                    url: "u".to_owned(),
                    label: vec![Span::Bold(plain(&["b"]))],
                }],
            ),
            ("[no url]", plain(&["[no url]"])),
            (
                "see https://ex.test/path_a now",
                vec![
                    t("see "),
                    Span::Autolink("https://ex.test/path_a".to_owned()),
                    t(" now"),
                ],
            ),
            (
                "go https://x.test/a.",
                vec![
                    t("go "),
                    Span::Autolink("https://x.test/a".to_owned()),
                    t("."),
                ],
            ),
            (
                "(visit http://a.b/c)",
                vec![
                    t("(visit "),
                    Span::Autolink("http://a.b/c".to_owned()),
                    t(")"),
                ],
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_inline(input, 0), expected, "input: {input:?}");
        }
    }

    #[test]
    fn adjacent_literal_text_merges_into_one_span() {
        assert_eq!(
            parse_inline(r"pre \* mid ~~post", 0),
            plain(&[r"pre * mid ~~post"])
        );
    }

    // ---------- blocks ----------

    fn para(text: &str) -> Block {
        Block::Para(parse_inline(text, 0))
    }

    #[test]
    fn block_table() {
        let cases: Vec<(&str, Vec<Block>)> = vec![
            ("one line", vec![para("one line")]),
            ("a\nb", vec![para("a\nb")]),
            ("first\n\nsecond", vec![para("first"), para("second")]),
            ("# h1", vec![Block::Heading(1, plain(&["h1"]))]),
            ("## h2", vec![Block::Heading(2, plain(&["h2"]))]),
            ("### h3", vec![Block::Heading(3, plain(&["h3"]))]),
            ("#### four", vec![para("#### four")]),
            ("#nospace", vec![para("#nospace")]),
            (
                "-# small text",
                vec![Block::Subtext(plain(&["small text"]))],
            ),
            (
                "- # not subtext",
                vec![Block::List {
                    ordered: false,
                    items: vec![parse_inline("# not subtext", 0)],
                }],
            ),
            (
                "```rust\nfn x() {}\n```",
                vec![Block::Code {
                    lang: "rust".to_owned(),
                    body: "fn x() {}".to_owned(),
                }],
            ),
            (
                "```\n**not bold**\n```",
                vec![Block::Code {
                    lang: String::new(),
                    body: "**not bold**".to_owned(),
                }],
            ),
            (
                "```js\nlet a = 1;",
                vec![Block::Code {
                    lang: "js".to_owned(),
                    body: "let a = 1;".to_owned(),
                }],
            ),
            (
                "> line1\n> line2",
                vec![Block::Quote {
                    wide: false,
                    blocks: vec![para("line1\nline2")],
                }],
            ),
            (
                "> # Title\n> body",
                vec![Block::Quote {
                    wide: false,
                    blocks: vec![Block::Heading(1, plain(&["Title"])), para("body")],
                }],
            ),
            (
                ">>> all\nof this is quoted",
                vec![Block::Quote {
                    wide: true,
                    blocks: vec![para("all\nof this is quoted")],
                }],
            ),
            (
                "- alpha\n- beta",
                vec![Block::List {
                    ordered: false,
                    items: vec![parse_inline("alpha", 0), parse_inline("beta", 0)],
                }],
            ),
            (
                "1. first\n2. second",
                vec![Block::List {
                    ordered: true,
                    items: vec![parse_inline("first", 0), parse_inline("second", 0)],
                }],
            ),
            (
                "- **big** item",
                vec![Block::List {
                    ordered: false,
                    items: vec![parse_inline("**big** item", 0)],
                }],
            ),
            (
                "# Title\nintro text\n- one\n- two\n\nafter gap",
                vec![
                    Block::Heading(1, plain(&["Title"])),
                    para("intro text"),
                    Block::List {
                        ordered: false,
                        items: vec![parse_inline("one", 0), parse_inline("two", 0)],
                    },
                    para("after gap"),
                ],
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(parse(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn deep_marker_nesting_degrades_to_text_instead_of_overflowing() {
        let input = "*".repeat(200) + "x";
        let blocks = parse(&input);
        assert!(matches!(blocks.as_slice(), [Block::Para(_)]));
    }

    #[test]
    fn fuzzish_inputs_never_panic() {
        struct XorShift(u64);
        impl XorShift {
            fn next(&mut self) -> u64 {
                let mut x = self.0;
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                self.0 = x;
                x
            }
        }
        let alphabet: Vec<char> = "*~_|<>[]()`\\#@&:!.-ab1 \n>=\"'".chars().collect();
        let mut rng = XorShift(0x2545_F491_4F6C_DD1D);
        for _case in 0..500 {
            let len = 1 + (rng.next() % 300) as usize;
            let input: String = (0..len)
                .map(|_| alphabet[(rng.next() % alphabet.len() as u64) as usize])
                .collect();
            parse(&input);
        }
    }

    #[test]
    fn multibyte_characters_survive_parsing() {
        assert_eq!(parse_inline("héllo wörld ✓", 0), plain(&["héllo wörld ✓"]));
        assert_eq!(parse("日本語\nテスト"), vec![para("日本語\nテスト")]);
    }
}
