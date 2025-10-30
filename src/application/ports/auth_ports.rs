use crate::common::errors::DomainError;
use crate::domain::entities::session::Session;
use crate::domain::entities::user::User;
use async_trait::async_trait;

#[async_trait]
pub trait UserStoragePort: Send + Sync + 'static {
    /// Crea un nuevo usuario
    async fn create_user(&self, user: User) -> Result<User, DomainError>;

    /// Obtiene un usuario por ID
    async fn get_user_by_id(&self, id: &str) -> Result<User, DomainError>;

    /// Obtiene un usuario por nombre de usuario
    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError>;

    /// Obtiene un usuario por correo electrónico
    async fn get_user_by_email(&self, email: &str) -> Result<User, DomainError>;

    /// Actualiza un usuario existente
    async fn update_user(&self, user: User) -> Result<User, DomainError>;

    /// Actualiza solo el uso de almacenamiento de un usuario
    async fn update_storage_usage(
        &self,
        user_id: &str,
        usage_bytes: i64,
    ) -> Result<(), DomainError>;

    /// Lista usuarios con paginación
    async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>, DomainError>;

    /// Lista usuarios por rol (por ejemplo, "admin" o "user")
    async fn list_users_by_role(&self, role: &str) -> Result<Vec<User>, DomainError>;

    /// Elimina un usuario por su ID
    async fn delete_user(&self, user_id: &str) -> Result<(), DomainError>;

    /// Cambia la contraseña de un usuario
    async fn change_password(&self, user_id: &str, password_hash: &str) -> Result<(), DomainError>;
}

#[async_trait]
pub trait SessionStoragePort: Send + Sync + 'static {
    /// Crea una nueva sesión
    async fn create_session(&self, session: Session) -> Result<Session, DomainError>;

    /// Obtiene una sesión por token de actualización
    async fn get_session_by_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<Session, DomainError>;

    /// Revoca una sesión específica
    async fn revoke_session(&self, session_id: &str) -> Result<(), DomainError>;

    /// Revoca todas las sesiones de un usuario
    async fn revoke_all_user_sessions(&self, user_id: &str) -> Result<u64, DomainError>;
}
