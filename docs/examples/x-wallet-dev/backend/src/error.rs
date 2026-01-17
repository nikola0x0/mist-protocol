use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum BackendError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Webhook validation failed: {0}")]
    WebhookValidation(String),

    #[error("Event already processed: {0}")]
    DuplicateEvent(String),

    #[error("Twitter API error: {0}")]
    TwitterApi(String),

    #[error("Enclave error: {0}")]
    Enclave(String),

    #[error("Sui network error: {0}")]
    SuiNetwork(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for BackendError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            BackendError::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            BackendError::Redis(ref e) => {
                tracing::error!("Redis error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error")
            }
            BackendError::Config(ref e) => {
                tracing::error!("Configuration error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error")
            }
            BackendError::WebhookValidation(ref e) => {
                tracing::warn!("Webhook validation failed: {}", e);
                (StatusCode::BAD_REQUEST, "Invalid webhook")
            }
            BackendError::DuplicateEvent(ref e) => {
                tracing::info!("Duplicate event: {}", e);
                (StatusCode::OK, "Already processed")
            }
            BackendError::TwitterApi(ref e) => {
                tracing::error!("Twitter API error: {}", e);
                (StatusCode::BAD_GATEWAY, "Twitter API error")
            }
            BackendError::Enclave(ref e) => {
                tracing::error!("Enclave error: {}", e);
                (StatusCode::BAD_GATEWAY, "Enclave error")
            }
            BackendError::SuiNetwork(ref e) => {
                tracing::error!("Sui network error: {}", e);
                (StatusCode::BAD_GATEWAY, "Blockchain error")
            }
            BackendError::Internal(ref e) => {
                tracing::error!("Internal error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
            }
        };

        let body = Json(json!({
            "error": error_message,
            "details": self.to_string(),
        }));

        (status, body).into_response()
    }
}

// Convenience type alias
pub type Result<T> = std::result::Result<T, BackendError>;

// Helper for converting anyhow errors
impl From<anyhow::Error> for BackendError {
    fn from(err: anyhow::Error) -> Self {
        BackendError::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== Error Display tests ======

    #[test]
    fn test_config_error_display() {
        let err = BackendError::Config("Missing API key".to_string());
        assert_eq!(err.to_string(), "Configuration error: Missing API key");
    }

    #[test]
    fn test_webhook_validation_error_display() {
        let err = BackendError::WebhookValidation("Invalid signature".to_string());
        assert_eq!(err.to_string(), "Webhook validation failed: Invalid signature");
    }

    #[test]
    fn test_duplicate_event_error_display() {
        let err = BackendError::DuplicateEvent("tweet:123".to_string());
        assert_eq!(err.to_string(), "Event already processed: tweet:123");
    }

    #[test]
    fn test_twitter_api_error_display() {
        let err = BackendError::TwitterApi("Rate limited".to_string());
        assert_eq!(err.to_string(), "Twitter API error: Rate limited");
    }

    #[test]
    fn test_enclave_error_display() {
        let err = BackendError::Enclave("Connection refused".to_string());
        assert_eq!(err.to_string(), "Enclave error: Connection refused");
    }

    #[test]
    fn test_sui_network_error_display() {
        let err = BackendError::SuiNetwork("Transaction failed".to_string());
        assert_eq!(err.to_string(), "Sui network error: Transaction failed");
    }

    #[test]
    fn test_internal_error_display() {
        let err = BackendError::Internal("Unexpected error".to_string());
        assert_eq!(err.to_string(), "Internal error: Unexpected error");
    }

    // ====== Error From conversions ======

    #[test]
    fn test_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("Test anyhow error");
        let backend_err: BackendError = anyhow_err.into();

        match backend_err {
            BackendError::Internal(msg) => assert_eq!(msg, "Test anyhow error"),
            _ => panic!("Expected Internal error variant"),
        }
    }

    #[test]
    fn test_error_from_anyhow_with_context() {
        let anyhow_err = anyhow::anyhow!("Root cause").context("Additional context");
        let backend_err: BackendError = anyhow_err.into();

        match backend_err {
            BackendError::Internal(msg) => {
                assert!(msg.contains("Additional context"));
            }
            _ => panic!("Expected Internal error variant"),
        }
    }

    // ====== Error Debug trait ======

    #[test]
    fn test_error_debug_format() {
        let err = BackendError::Config("Test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("Test"));
    }
}
