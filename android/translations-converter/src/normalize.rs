use regex::Regex;
use std::sync::LazyLock;

pub trait Normalize {
    /// Normalize the string value into a common format.
    ///
    /// Makes it possible to compare different representations of translation messages.
    fn normalize(&self) -> String;
}

mod android {
    use super::*;
    use crate::android::StringValue;

    static APOSTROPHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\'").unwrap());
    static DOUBLE_QUOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\\""#).unwrap());
    static PARAMETERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%[0-9]*\$").unwrap());

    impl Normalize for StringValue {
        fn normalize(&self) -> String {
            // Unescape apostrophes
            let value = APOSTROPHES.replace_all(self, "'");
            // Unescape double quotes
            let value = DOUBLE_QUOTES.replace_all(&value, r#"""#);
            // Mark where parameters are positioned, removing the parameter index
            let value = PARAMETERS.replace_all(&value, "%");

            // Unescape XML characters
            htmlize::unescape(value).into()
        }
    }

    impl StringValue {
        pub fn normalize_keep_parameter_indices(&self) -> String {
            // Unescape apostrophes
            let value = APOSTROPHES.replace_all(self, "'");
            // Unescape double quotes
            let value = DOUBLE_QUOTES.replace_all(&value, r#"""#);

            // Unescape XML characters
            htmlize::unescape(value).into()
        }
    }
}

mod gettext {
    use super::*;
    use crate::gettext::MsgString;

    static ESCAPED_SINGLE_QUOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\'").unwrap());
    static ESCAPED_DOUBLE_QUOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\\""#).unwrap());
    // Look for both %(...) and %(1-9)$(...) as we now push the latter to the common translation file
    static NAMED_PARAMETERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%\([^)]*\)").unwrap());
    static ORDERED_PARAMETERS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"%[0-9]*\$").unwrap());

    impl Normalize for MsgString {
        fn normalize(&self) -> String {
            // Mark where parameters are positioned, removing the parameter name
            let string = NAMED_PARAMETERS.replace_all(self, "%");
            let string = ORDERED_PARAMETERS.replace_all(&string, "%");
            // Remove escaped single-quotes
            let string = ESCAPED_SINGLE_QUOTES.replace_all(&string, r"'");
            // Remove escaped double-quotes
            let string = ESCAPED_DOUBLE_QUOTES.replace_all(&string, r#"""#);

            string.into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Normalize;

    #[test]
    fn normalize_android_string_value() {
        use crate::android::StringValue;

        let input = StringValue::from_unescaped(
            concat!(
                "'Inside single quotes'",
                r#""Inside double quotes""#,
                "With parameters: %1$d, %2$s",
            ),
            None,
        );

        let expected = concat!(
            "'Inside single quotes'",
            r#""Inside double quotes""#,
            "With parameters: %d, %s",
        );

        assert_eq!(input.normalize(), expected);
    }

    #[test]
    fn normalize_gettext_msg_string() {
        use crate::gettext::MsgString;

        let input = MsgString::from_escaped(concat!(
            "'Inside single quotes'",
            r"\'Inside escaped single quotes\'",
            r#"\"Inside double quotes\""#,
            "With parameters: %(number)d, %(string)s",
        ));

        let expected = concat!(
            "'Inside single quotes'",
            "'Inside escaped single quotes'",
            r#""Inside double quotes""#,
            "With parameters: %d, %s",
        );

        assert_eq!(input.normalize(), expected);
    }
}
