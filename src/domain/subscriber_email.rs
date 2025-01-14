use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        s.validate_email()
            .then_some(Self(s))
            .ok_or("Invalid email".to_string())
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);
    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }
    #[test]
    fn a_valid_email_are_ok() {
        let email = SafeEmail().fake();
        claims::assert_ok!(SubscriberEmail::parse(email));
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_ok(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
