use crate::common::errors::DomainError;
use crate::domain::entities::session::Session;
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum SessionRepositoryError {
    #[error("Sesión no encontrada: {0}")]
    NotFound(String),

    #[error("Error de base de datos: {0}")]
    DatabaseError(String),

    #[error("Error de tiempo de espera: {0}")]
    Timeout(String),
}

pub type SessionRepositoryResult<T> = Result<T, SessionRepositoryError>;

// Conversión de SessionRepositoryError a DomainError
impl From<SessionRepositoryError> for DomainError {
    fn from(err: SessionRepositoryError) -> Self {
        match err {
            SessionRepositoryError::NotFound(msg) => DomainError::not_found("Session", msg),
            SessionRepositoryError::DatabaseError(msg) => {
                DomainError::internal_error("Database", msg)
            }
            SessionRepositoryError::Timeout(msg) => DomainError::timeout("Database", msg),
        }
    }
}

#[async_trait]
pub trait SessionRepository: Send + Sync + 'static {
    /// Crea una nueva sesión
    async fn create_session(&self, session: Session) -> SessionRepositoryResult<Session>;

    /// Obtiene una sesión por ID
    async fn get_session_by_id(&self, id: &str) -> SessionRepositoryResult<Session>;

    /// Obtiene una sesión por token de actualización
    async fn get_session_by_refresh_token(
        &self,
        refresh_token: &str,
    ) -> SessionRepositoryResult<Session>;

    /// Obtiene todas las sesiones de un usuario
    async fn get_sessions_by_user_id(&self, user_id: &str)
        -> SessionRepositoryResult<Vec<Session>>;

    /// Revoca una sesión específica
    async fn revoke_session(&self, session_id: &str) -> SessionRepositoryResult<()>;

    /// Revoca todas las sesiones de un usuario
    async fn revoke_all_user_sessions(&self, user_id: &str) -> SessionRepositoryResult<u64>;

    /// Elimina sesiones expiradas
    async fn delete_expired_sessions(&self) -> SessionRepositoryResult<u64>;
}
