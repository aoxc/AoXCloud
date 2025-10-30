use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Username inválido: {0}")]
    InvalidUsername(String),

    #[error("Password inválido: {0}")]
    InvalidPassword(String),

    #[error("Error en la validación: {0}")]
    ValidationError(String),

    #[error("Error en la autenticación: {0}")]
    AuthenticationError(String),
}

pub type UserResult<T> = Result<T, UserError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
// We'll handle conversion manually for now until the type is properly set up in the database
pub enum UserRole {
    Admin,
    User,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::User => write!(f, "user"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: String,
    username: String,
    email: String,
    #[serde(skip_serializing)]
    password_hash: String,
    role: UserRole,
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
    active: bool,
}

impl User {
    pub fn new(
        username: String,
        email: String,
        password: String,
        role: UserRole,
        storage_quota_bytes: i64,
    ) -> UserResult<Self> {
        // Validaciones
        if username.is_empty() || username.len() < 3 || username.len() > 32 {
            return Err(UserError::InvalidUsername(format!(
                "Username debe tener entre 3 y 32 caracteres"
            )));
        }

        if !email.contains('@') || email.len() < 5 {
            return Err(UserError::ValidationError(format!("Email inválido")));
        }

        if password.len() < 8 {
            return Err(UserError::InvalidPassword(format!(
                "Password debe tener al menos 8 caracteres"
            )));
        }

        // Generar hash con Argon2id (recomendado para 2023+)
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| UserError::ValidationError(format!("Error al generar hash: {}", e)))?
            .to_string();

        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            username,
            email,
            password_hash,
            role,
            storage_quota_bytes,
            storage_used_bytes: 0,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            active: true,
        })
    }

    // Crear desde valores existentes (para reconstrucción desde BD)
    pub fn from_data(
        id: String,
        username: String,
        email: String,
        password_hash: String,
        role: UserRole,
        storage_quota_bytes: i64,
        storage_used_bytes: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        last_login_at: Option<DateTime<Utc>>,
        active: bool,
    ) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            role,
            storage_quota_bytes,
            storage_used_bytes,
            created_at,
            updated_at,
            last_login_at,
            active,
        }
    }

    // Getters
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn role(&self) -> UserRole {
        self.role
    }

    pub fn storage_quota_bytes(&self) -> i64 {
        self.storage_quota_bytes
    }

    pub fn storage_used_bytes(&self) -> i64 {
        self.storage_used_bytes
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    // Verificación de password
    pub fn verify_password(&self, password: &str) -> UserResult<bool> {
        let parsed_hash = PasswordHash::new(&self.password_hash).map_err(|e| {
            UserError::AuthenticationError(format!("Error al procesar hash: {}", e))
        })?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    // Cambiar contraseña
    pub fn update_password(&mut self, new_password: String) -> UserResult<()> {
        if new_password.len() < 8 {
            return Err(UserError::InvalidPassword(format!(
                "Password debe tener al menos 8 caracteres"
            )));
        }

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        self.password_hash = argon2
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| UserError::ValidationError(format!("Error al generar hash: {}", e)))?
            .to_string();

        self.updated_at = Utc::now();
        Ok(())
    }

    // Actualizar uso de almacenamiento
    pub fn update_storage_used(&mut self, storage_used_bytes: i64) {
        self.storage_used_bytes = storage_used_bytes;
        self.updated_at = Utc::now();
    }

    // Registrar login
    pub fn register_login(&mut self) {
        let now = Utc::now();
        self.last_login_at = Some(now);
        self.updated_at = now;
    }

    // Desactivar usuario
    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = Utc::now();
    }

    // Activar usuario
    pub fn activate(&mut self) {
        self.active = true;
        self.updated_at = Utc::now();
    }
}
