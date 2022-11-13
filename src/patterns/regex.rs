use crate::patterns::{Pattern, Patterns};
use regex::{Regex, RegexBuilder};

impl Pattern for Regex {
    fn matches(&self, string: &str) -> bool {
        self.is_match(string)
    }

    fn parse<T: AsRef<str>>(string: T) -> Result<Self, String> {
        match RegexBuilder::new(string.as_ref())
            .case_insensitive(true)
            .multi_line(false)
            .dot_matches_new_line(false)
            .ignore_whitespace(true)
            .unicode(true)
            .build()
        {
            Ok(result) => Ok(result),
            Err(error) => Err(format!("Invalid regex: {}", error)),
        }
    }
}

pub struct RegexPatterns {
    vec: Vec<Regex>,
}

impl RegexPatterns {
    pub fn new(patterns: &[String]) -> Self {
        Self {
            vec: patterns
                .iter()
                .flat_map(|pattern| Regex::new(pattern))
                .collect(),
        }
    }
}

impl Patterns for RegexPatterns {
    fn contains(&self, address: impl AsRef<str>) -> bool {
        // Linear search
        for pattern in &self.vec {
            if pattern.matches(address.as_ref()) {
                return true;
            }
        }

        false
    }

    fn len(&self) -> usize {
        self.vec.len()
    }
}
