use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::errors::{DomainError, ErrorKind};
use crate::domain::entities::user::User;

/**
 * JWT claims structure for authentication tokens.
 *
 * This structure represents the payload of JWT tokens used for authentication
 * in the system. It contains all the necessary claims to identify users and
 * manage token lifecycle.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject identifier - contains the user ID
    pub sub: String,

    /// Expiration timestamp (seconds since Unix epoch)
    pub exp: i64,

    /// Issued at timestamp (seconds since Unix epoch)
    pub iat: i64,

    /// JWT unique ID for token tracking and revocation
    pub jti: String,

    /// Username for display and identification purposes
    pub username: String,

    /// User email for communication and identification
    pub email: String,

    /// User role for authorization checks
    pub role: String,
}

/**
 * Authentication-specific error types.
 *
 * This enum encapsulates all error scenarios that can occur during
 * authentication and authorization processes, providing clear error messages
 * and categorization for proper handling.
 */
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Returned when username/password authentication fails
    #[error("Credenciales inválidas")]
    InvalidCredentials,

    /// Returned when a JWT token has passed its expiration time
    #[error("Token expirado")]
    TokenExpired,

    /// Returned when a JWT token is malformed or has invalid signature
    #[error("Token inválido: {0}")]
    InvalidToken(String),

    /// Returned when a user attempts to access a resource they don't have permission for
    #[error("Acceso denegado: {0}")]
    AccessDenied(String),

    /// Returned when a requested operation is not allowed for the user
    #[error("Operación no permitida: {0}")]
    OperationNotAllowed(String),

    /// Returned for unexpected errors in the authentication system
    #[error("Error interno: {0}")]
    InternalError(String),
}

/**
 * Conversion from AuthError to DomainError.
 *
 * This implementation allows authentication errors to be seamlessly
 * transformed into domain errors, making error handling more consistent
 * throughout the application.
 */
impl From<AuthError> for DomainError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidCredentials => {
                DomainError::new(ErrorKind::AccessDenied, "Auth", "Credenciales inválidas")
            }
            AuthError::TokenExpired => {
                DomainError::new(ErrorKind::AccessDenied, "Auth", "Token expirado")
            }
            AuthError::InvalidToken(msg) => DomainError::new(
                ErrorKind::AccessDenied,
                "Auth",
                format!("Token inválido: {}", msg),
            ),
            AuthError::AccessDenied(msg) => DomainError::new(ErrorKind::AccessDenied, "Auth", msg),
            AuthError::OperationNotAllowed(msg) => {
                DomainError::new(ErrorKind::AccessDenied, "Auth", msg)
            }
            AuthError::InternalError(msg) => {
                DomainError::new(ErrorKind::InternalError, "Auth", msg)
            }
        }
    }
}

/**
 * Authentication service for managing user sessions and authorization.
 *
 * This service provides the core authentication functionality for the system,
 * including JWT token generation and validation, refresh token management,
 * and session duration control.
 */
pub struct AuthService {
    /// Secret key used for signing JWT tokens
    jwt_secret: String,

    /// Expiration time for access tokens in seconds
    access_token_expiry: i64,

    /// Expiration time for refresh tokens in seconds
    refresh_token_expiry: i64,
}

impl AuthService {
    pub fn new(
        jwt_secret: String,
        access_token_expiry_secs: i64,
        refresh_token_expiry_secs: i64,
    ) -> Self {
        Self {
            jwt_secret,
            access_token_expiry: access_token_expiry_secs,
            refresh_token_expiry: refresh_token_expiry_secs,
        }
    }

    pub fn generate_access_token(&self, user: &User) -> Result<String, AuthError> {
        let now = Utc::now().timestamp();

        // Log information for debugging
        tracing::debug!(
            "Generating token for user: {}, id: {}, role: {}",
            user.username(),
            user.id(),
            user.role()
        );

        let claims = TokenClaims {
            sub: user.id().to_string(),
            exp: now + self.access_token_expiry,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            username: user.username().to_string(),
            email: user.email().to_string(),
            role: format!("{}", user.role()),
        };

        // Log JWT claims for debugging
        tracing::debug!(
            "JWT claims: sub={}, exp={}, iat={}",
            claims.sub,
            claims.exp,
            claims.iat
        );

        match encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        ) {
            Ok(token) => {
                tracing::debug!("Token generated successfully, length: {}", token.len());
                Ok(token)
            }
            Err(e) => {
                tracing::error!("Error generating token: {}", e);
                Err(AuthError::InternalError(format!(
                    "Error al generar token: {}",
                    e
                )))
            }
        }
    }

    pub fn generate_refresh_token(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<TokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken(format!("Error al validar token: {}", e)),
        })?;

        Ok(token_data.claims)
    }

    // Duración del refresh token en segundos
    pub fn refresh_token_expiry_secs(&self) -> i64 {
        self.refresh_token_expiry
    }

    // Duración del refresh token en días (para la entidad Session)
    pub fn refresh_token_expiry_days(&self) -> i64 {
        self.refresh_token_expiry / (24 * 3600)
    }
}
