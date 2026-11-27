//! Shape checks shared by every crate that validates automation wire data.
//!
//! These are the single copy of the rules for identifiers, decimal integers
//! and digests: a protocol fix lands here once instead of in each validator.

/// A non-empty identifier of at most `max` bytes with no surrounding
/// whitespace, no `*` wildcard and no control characters.
pub fn canonical_id(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}

/// A base-10 `i64` in its canonical spelling (no sign, padding or `+`),
/// at least `1` when `positive`, otherwise at least `0`.
pub fn decimal(value: &str, positive: bool) -> Option<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= i64::from(positive) && n.to_string() == value)
}

/// A lowercase hexadecimal SHA-256 digest (64 characters).
pub fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_id_rejects_empty_padded_wildcard_control_and_oversized() {
        assert!(canonical_id("abc", 3));
        for bad in ["", " a", "a ", "a*", "a\nb"] {
            assert!(!canonical_id(bad, 8), "{bad:?}");
        }
        assert!(!canonical_id("abcd", 3));
    }

    #[test]
    fn decimal_accepts_only_the_canonical_spelling() {
        assert_eq!(decimal("0", false), Some(0));
        assert_eq!(decimal("0", true), None);
        assert_eq!(decimal("12", true), Some(12));
        for bad in ["01", "+1", "-1", " 1", "1.0", ""] {
            assert_eq!(decimal(bad, false), None, "{bad:?}");
        }
    }

    #[test]
    fn digest_requires_64_lowercase_hex_characters() {
        assert!(digest(&"a0".repeat(32)));
        assert!(!digest(&"A0".repeat(32)));
        assert!(!digest(&"a".repeat(63)));
        assert!(!digest(&"g".repeat(64)));
    }
}
