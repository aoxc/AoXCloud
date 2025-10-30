use async_trait::async_trait;
use thiserror::Error;

/// Error types for i18n service operations
#[derive(Debug, Error)]
pub enum I18nError {
    #[error("Translation key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid locale: {0}")]
    InvalidLocale(String),

    #[error("Error loading translations: {0}")]
    LoadError(String),
}

/// Result type for i18n service operations
pub type I18nResult<T> = Result<T, I18nError>;

/// Supported locales
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    English,
    Spanish,
}

impl Locale {
    /// Convert locale to code string
    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::English => "en",
            Locale::Spanish => "es",
        }
    }

    /// Create from locale code string
    pub fn from_str(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" => Some(Locale::English),
            "es" => Some(Locale::Spanish),
            _ => None,
        }
    }

    /// Get default locale
    pub fn default() -> Self {
        Locale::English
    }
}

/// Interface for i18n service (primary port)
#[async_trait]
pub trait I18nService: Send + Sync + 'static {
    /// Get a translation for a key and locale
    async fn translate(&self, key: &str, locale: Locale) -> I18nResult<String>;

    /// Load translations for a locale
    async fn load_translations(&self, locale: Locale) -> I18nResult<()>;

    /// Get available locales
    async fn available_locales(&self) -> Vec<Locale>;

    /// Check if a locale is supported
    #[allow(dead_code)]
    async fn is_supported(&self, locale: Locale) -> bool;
}
