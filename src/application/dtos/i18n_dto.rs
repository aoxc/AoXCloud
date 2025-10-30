use crate::domain::services::i18n_service::Locale;
use serde::{Deserialize, Serialize};

/// DTO for locale information
#[derive(Debug, Serialize, Deserialize)]
pub struct LocaleDto {
    /// Locale code (e.g., "en", "es")
    pub code: String,

    /// Locale name in its own language (e.g., "English", "Español")
    pub name: String,
}

impl From<Locale> for LocaleDto {
    fn from(locale: Locale) -> Self {
        let (code, name) = match locale {
            Locale::English => ("en", "English"),
            Locale::Spanish => ("es", "Español"),
        };

        Self {
            code: code.to_string(),
            name: name.to_string(),
        }
    }
}

/// DTO for translation request
#[derive(Debug, Deserialize)]
pub struct TranslationRequestDto {
    /// The translation key
    pub key: String,

    /// The locale code (optional, defaults to "en")
    pub locale: Option<String>,
}

/// DTO for translation response
#[derive(Debug, Serialize)]
pub struct TranslationResponseDto {
    /// The translation key
    pub key: String,

    /// The locale code used for translation
    pub locale: String,

    /// The translated text
    pub text: String,
}

/// DTO for translation error
#[derive(Debug, Serialize)]
pub struct TranslationErrorDto {
    /// The translation key that was not found
    pub key: String,

    /// The locale code used for translation
    pub locale: String,

    /// The error message
    pub error: String,
}
