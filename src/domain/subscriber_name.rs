use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|c| forbidden_characters.contains(&c));
        if is_empty || is_too_long || contains_forbidden_characters {
            Err(format!("{} is not a valid Subscriber name", s))
        } else {
            Ok(Self(s))
        }
    }
}
impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ã".repeat(256);
        claims::assert_ok!(SubscriberName::parse(name));
    }
    #[test]
    fn a_name_longer_than_256_grapheme_is_error() {
        let name = "ã".repeat(257);
        claims::assert_err!(SubscriberName::parse(name));
    }
    #[test]
    fn an_empty_name_is_error() {
        claims::assert_err!(SubscriberName::parse("  ".to_string()));
    }
    #[test]
    fn a_name_containing_forbidden_character_is_error() {
        let test_cases = [
            ("abcd{", "{"),
            ("}bcdf", "}"),
            ("<bcdf", "<"),
            ("a>cdf", ">"),
        ];
        for (error_name, description) in test_cases {
            claims::assert_err!(
                SubscriberName::parse(error_name.to_string()),
                "Name with '{description}' should not be ok"
            );
        }
    }
    #[test]
    fn a_valid_name_is_ok() {
        claims::assert_ok!(SubscriberName::parse("Ursula Guain".to_string()));
    }
}
