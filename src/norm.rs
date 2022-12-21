use regex::Regex;

#[derive(Debug, Clone)]
pub struct Normalizer {
    regex_non_filing: Regex,
    regex_apos: Regex,
    regex_quote: Regex,
}

impl Normalizer {

    pub fn new() -> Normalizer {
        Normalizer {

            // Non-filing strings
            regex_non_filing: Regex::new(r#"\x{0098}.*?\x{009C}"#).unwrap(),

            // Single quote-like characters
            regex_apos:Regex::new(r#"[\x{2018}\x{2019}\x{201B}\x{FF07}\x{201A}]"#).unwrap(),

            // Quote-like characters
            regex_quote: Regex::new(r#"[\x{201C}\x{201D}\x{201F}\x{FF0C}\x{201E}\x{2E42}]"#).unwrap(),
        }
    }


    // See Evergreen/Open-ILS/src/perlmods/lib/OpenILS/Utils/Normalize.pm

    pub fn naco_normalize(&self, value: &str) -> Result<String, String> {

        let value = self.normalize_substitutions(value)?;

        Ok(value)
    }

    fn normalize_substitutions(&self, value: &str) -> Result<String, String> {

        let mut value = value.to_uppercase();

        value = self.regex_non_filing.replace_all(&value, "").into_owned();
        value = self.regex_apos.replace_all(&value, "'").into_owned();
        value = self.regex_quote.replace_all(&value, "\"").into_owned();

        /*
        $str = NFKD($str);

        # additional substitutions - 3.6.
        $str =~ s/\x{00C6}/AE/g;
        $str =~ s/\x{00DE}/TH/g;
        $str =~ s/\x{0152}/OE/g;
        $str =~ tr/\x{0110}\x{00D0}\x{00D8}\x{0141}\x{2113}\x{02BB}\x{02BC}][/DDOLl/d;
        */

        Ok(value)

    }
}
