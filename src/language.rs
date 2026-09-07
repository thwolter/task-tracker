//! Runtime language selection for translated UI and report text.
//!
//! German is selected only when the system locale begins with `de`; all other
//! or unavailable locales use English.

/// A supported application language.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Language {
    English,
    German,
}

impl Language {
    /// Selects a supported language from the operating system locale.
    pub(crate) fn system() -> Self {
        Self::from_locale(sys_locale::get_locale().as_deref())
    }

    /// Returns the bundled Slint translation locale for this language.
    #[cfg(not(feature = "live-preview"))]
    pub(crate) fn slint_locale(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
        }
    }

    fn from_locale(locale: Option<&str>) -> Self {
        locale
            .is_some_and(|locale| {
                locale
                    .split(['-', '_', '@'])
                    .next()
                    .is_some_and(|language| language.eq_ignore_ascii_case("de"))
            })
            .then_some(Self::German)
            .unwrap_or(Self::English)
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn german_locales_select_german_and_other_locales_fall_back_to_english() {
        assert_eq!(Language::from_locale(Some("de-DE")), Language::German);
        assert_eq!(Language::from_locale(Some("de_AT")), Language::German);
        assert_eq!(Language::from_locale(Some("de")), Language::German);
        assert_eq!(Language::from_locale(Some("de@euro")), Language::German);
        assert_eq!(Language::from_locale(Some("en-US")), Language::English);
        assert_eq!(Language::from_locale(None), Language::English);
    }
}
