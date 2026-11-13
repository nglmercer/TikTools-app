//! Small, bounded IRCv3 parser. Wire tags are data, never commands or authority.
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub(crate) struct Message<'a> {
    pub tags: BTreeMap<&'a str, String>,
    pub prefix: Option<&'a str>,
    pub command: &'a str,
    pub target: Option<&'a str>,
    pub trailing: Option<&'a str>,
}
pub(crate) fn parse(line: &str) -> Option<Message<'_>> {
    if line.len() > 64 * 1024 {
        return None;
    }
    let mut rest = line.trim_end_matches('\r');
    let mut tags = BTreeMap::new();
    if let Some(tagged) = rest.strip_prefix('@') {
        let (raw, next) = tagged.split_once(' ')?;
        for pair in raw.split(';').take(128) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            tags.insert(key, unescape(value));
        }
        rest = next;
    }
    let prefix = if let Some(prefixed) = rest.strip_prefix(':') {
        let (prefix, next) = prefixed.split_once(' ')?;
        rest = next;
        Some(prefix)
    } else {
        None
    };
    let (middle, trailing) = rest
        .split_once(" :")
        .map_or((rest, None), |(a, b)| (a, Some(b)));
    let mut words = middle.split_whitespace();
    Some(Message {
        tags,
        prefix,
        command: words.next()?,
        target: words.next(),
        trailing,
    })
}
fn unescape(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some(':') => out.push(';'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some(c) => out.push(c),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_tags_and_preserves_chat_colons() {
        let m = parse("@id=abc;display-name=Hello\\sWorld;escaped=a\\:b\\\\c :alice!alice@host PRIVMSG #room :hi : there\r").unwrap();
        assert_eq!(m.command, "PRIVMSG");
        assert_eq!(m.target, Some("#room"));
        assert_eq!(m.trailing, Some("hi : there"));
        assert_eq!(m.tags["display-name"], "Hello World");
        assert_eq!(m.tags["escaped"], "a;b\\c");
    }
    #[test]
    fn parses_ping_and_rejects_oversized_lines() {
        assert_eq!(
            parse("PING :tmi.twitch.tv").unwrap().trailing,
            Some("tmi.twitch.tv")
        );
        assert!(parse(&"x".repeat(65537)).is_none());
        assert!(parse("@no-command").is_none());
    }
}
