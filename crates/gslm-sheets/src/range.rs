//! A1-notation range strings for a whole Tab, and their URL path encoding.

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

/// Characters that must be escaped inside a URL path segment
/// (RFC 3986 pchar complement, plus `'` which Google accepts either way).
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'\'')
    .add(b'!');

/// Quote a tab name for A1 notation: always single-quoted, inner `'` doubled.
pub fn quote_tab(tab: &str) -> String {
    format!("'{}'", tab.replace('\'', "''"))
}

/// The range covering the whole tab, e.g. `'My Tab'`.
pub fn whole_tab_range(tab: &str) -> String {
    quote_tab(tab)
}

/// The range anchoring a write at the top-left cell, e.g. `'My Tab'!A1`.
pub fn a1_range(tab: &str) -> String {
    format!("{}!A1", quote_tab(tab))
}

/// Percent-encode a range for use as a URL path segment.
pub fn encode_path_segment(range: &str) -> String {
    utf8_percent_encode(range, PATH_SEGMENT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_plain_names() {
        assert_eq!(quote_tab("i18n"), "'i18n'");
        assert_eq!(a1_range("i18n"), "'i18n'!A1");
    }

    #[test]
    fn doubles_inner_single_quotes() {
        assert_eq!(quote_tab("it's"), "'it''s'");
    }

    #[test]
    fn encodes_spaces_quotes_and_unicode() {
        assert_eq!(
            encode_path_segment(&whole_tab_range("翻譯 (v2)")),
            "%27%E7%BF%BB%E8%AD%AF%20(v2)%27"
        );
        assert_eq!(encode_path_segment("'a'!A1"), "%27a%27%21A1");
    }

    #[test]
    fn keeps_slash_out_of_segment() {
        assert_eq!(encode_path_segment("'a/b'"), "%27a%2Fb%27");
    }
}
