use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("No provider configured")]
    NoProvider,

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ProviderError>;

impl From<anyhow::Error> for ProviderError {
    fn from(e: anyhow::Error) -> Self {
        ProviderError::Other(e.to_string())
    }
}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            ProviderError::Timeout
        } else if e.is_connect() {
            ProviderError::NetworkError(e.to_string())
        } else {
            ProviderError::NetworkError(e.to_string())
        }
    }
}
