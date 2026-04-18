pub const DEFAULT_TOPIC: &str = "twenty-questions";
pub const TOPIC_KEY: &str = "topic";

/// Returns true if `s` is a valid NATS publish subject.
///
/// Rules: non-empty, printable ASCII only, tokens separated by '.' where every
/// token is non-empty, and no wildcard characters ('*' or '>') which are only
/// valid on subscription subjects.
pub fn is_valid_nats_subject(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    for token in s.split('.') {
        if token.is_empty() {
            return false;
        }
        for ch in token.chars() {
            match ch {
                '*' | '>' => return false,
                c if c.is_ascii() && !c.is_ascii_control() && c != ' ' => {}
                _ => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_subjects() {
        for s in &[
            "foo",
            "twenty-questions",
            "foo.bar",
            "foo.bar.baz",
            "a.b.c.d",
            "FOO.BAR",
            "foo123.bar_baz",
        ] {
            assert!(is_valid_nats_subject(s), "{s:?} should be valid");
        }
    }

    #[test]
    fn invalid_subjects() {
        for s in &[
            "",
            ".",
            ".foo",
            "foo.",
            "foo..bar",
            "foo bar",
            "foo\tbar",
            "foo\nbar",
            "foo.*",
            "foo.>",
            "*",
            ">",
            "foo.*.bar",
        ] {
            assert!(!is_valid_nats_subject(s), "{s:?} should be invalid");
        }
    }

    #[test]
    fn default_topic_is_valid() {
        assert!(is_valid_nats_subject(DEFAULT_TOPIC));
    }
}
