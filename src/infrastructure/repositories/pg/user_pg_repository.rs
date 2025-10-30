use async_trait::async_trait;
use futures::future::BoxFuture;
use sqlx::{PgPool, Row};
use std::sync::Arc;

use crate::application::ports::auth_ports::UserStoragePort;
use crate::common::errors::DomainError;
use crate::domain::entities::user::{User, UserRole};
use crate::domain::repositories::user_repository::{
    UserRepository, UserRepositoryError, UserRepositoryResult,
};
use crate::infrastructure::repositories::pg::transaction_utils::with_transaction;

// Implementar From<sqlx::Error> para UserRepositoryError para permitir conversiones automáticas
impl From<sqlx::Error> for UserRepositoryError {
    fn from(err: sqlx::Error) -> Self {
        UserPgRepository::map_sqlx_error(err)
    }
}

pub struct UserPgRepository {
    pool: Arc<PgPool>,
}

impl UserPgRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    // Método auxiliar para mapear errores SQL a errores de dominio
    pub fn map_sqlx_error(err: sqlx::Error) -> UserRepositoryError {
        match err {
            sqlx::Error::RowNotFound => {
                UserRepositoryError::NotFound("Usuario no encontrado".to_string())
            }
            sqlx::Error::Database(db_err) => {
                if db_err.code().map_or(false, |code| code == "23505") {
                    // Código para violación de unicidad en PostgreSQL
                    UserRepositoryError::AlreadyExists("Usuario o email ya existe".to_string())
                } else {
                    UserRepositoryError::DatabaseError(format!(
                        "Error de base de datos: {}",
                        db_err
                    ))
                }
            }
            _ => UserRepositoryError::DatabaseError(format!("Error de base de datos: {}", err)),
        }
    }
}

#[async_trait]
impl UserRepository for UserPgRepository {
    /// Crea un nuevo usuario utilizando una transacción
    async fn create_user(&self, user: User) -> UserRepositoryResult<User> {
        // Creamos una copia del usuario para el closure
        let user_clone = user.clone();

        with_transaction(&self.pool, "create_user", |tx| {
            // Necesitamos mover el closure a un BoxFuture para devolver dentro
            // de la llamada with_transaction
            Box::pin(async move {
                // Usamos los getters para extraer los valores
                // Convertimos user.role() a string para pasarlo como texto plano
                let role_str = user_clone.role().to_string();

                // Modificar el SQL para hacer un cast explícito al tipo auth.userrole
                let _result = sqlx::query(
                    r#"
                        INSERT INTO auth.users (
                            id, username, email, password_hash, role, 
                            storage_quota_bytes, storage_used_bytes, 
                            created_at, updated_at, last_login_at, active
                        ) VALUES (
                            $1, $2, $3, $4, $5::auth.userrole, $6, $7, $8, $9, $10, $11
                        )
                        RETURNING *
                        "#,
                )
                .bind(user_clone.id())
                .bind(user_clone.username())
                .bind(user_clone.email())
                .bind(user_clone.password_hash())
                .bind(&role_str) // Convertir a string pero con cast explícito en SQL
                .bind(user_clone.storage_quota_bytes())
                .bind(user_clone.storage_used_bytes())
                .bind(user_clone.created_at())
                .bind(user_clone.updated_at())
                .bind(user_clone.last_login_at())
                .bind(user_clone.is_active())
                .execute(&mut **tx)
                .await
                .map_err(Self::map_sqlx_error)?;

                // Podríamos realizar operaciones adicionales aquí,
                // como configurar permisos, roles, etc.

                Ok(user_clone)
            }) as BoxFuture<'_, UserRepositoryResult<User>>
        })
        .await?;

        Ok(user) // Devolvemos el usuario original por simplicidad
    }

    /// Obtiene un usuario por ID
    async fn get_user_by_id(&self, id: &str) -> UserRepositoryResult<User> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, username, email, password_hash, role::text as role_text, 
                storage_quota_bytes, storage_used_bytes, 
                created_at, updated_at, last_login_at, active
            FROM auth.users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        // Convert role string to UserRole enum
        let role_str: Option<String> = row.try_get("role_text").unwrap_or(None);
        let role = match role_str.as_deref() {
            Some("admin") => UserRole::Admin,
            _ => UserRole::User,
        };

        Ok(User::from_data(
            row.get("id"),
            row.get("username"),
            row.get("email"),
            row.get("password_hash"),
            role,
            row.get("storage_quota_bytes"),
            row.get("storage_used_bytes"),
            row.get("created_at"),
            row.get("updated_at"),
            row.get("last_login_at"),
            row.get("active"),
        ))
    }

    /// Obtiene un usuario por nombre de usuario
    async fn get_user_by_username(&self, username: &str) -> UserRepositoryResult<User> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, username, email, password_hash, role::text as role_text, 
                storage_quota_bytes, storage_used_bytes, 
                created_at, updated_at, last_login_at, active
            FROM auth.users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_one(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        // Convert role string to UserRole enum
        let role_str: Option<String> = row.try_get("role_text").unwrap_or(None);
        let role = match role_str.as_deref() {
            Some("admin") => UserRole::Admin,
            _ => UserRole::User,
        };

        Ok(User::from_data(
            row.get("id"),
            row.get("username"),
            row.get("email"),
            row.get("password_hash"),
            role,
            row.get("storage_quota_bytes"),
            row.get("storage_used_bytes"),
            row.get("created_at"),
            row.get("updated_at"),
            row.get("last_login_at"),
            row.get("active"),
        ))
    }

    /// Obtiene un usuario por correo electrónico
    async fn get_user_by_email(&self, email: &str) -> UserRepositoryResult<User> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, username, email, password_hash, role::text as role_text, 
                storage_quota_bytes, storage_used_bytes, 
                created_at, updated_at, last_login_at, active
            FROM auth.users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        // Convert role string to UserRole enum
        let role_str: Option<String> = row.try_get("role_text").unwrap_or(None);
        let role = match role_str.as_deref() {
            Some("admin") => UserRole::Admin,
            _ => UserRole::User,
        };

        Ok(User::from_data(
            row.get("id"),
            row.get("username"),
            row.get("email"),
            row.get("password_hash"),
            role,
            row.get("storage_quota_bytes"),
            row.get("storage_used_bytes"),
            row.get("created_at"),
            row.get("updated_at"),
            row.get("last_login_at"),
            row.get("active"),
        ))
    }

    /// Actualiza un usuario existente utilizando una transacción
    async fn update_user(&self, user: User) -> UserRepositoryResult<User> {
        // Creamos una copia del usuario para el closure
        let user_clone = user.clone();

        with_transaction(&self.pool, "update_user", |tx| {
            Box::pin(async move {
                // Actualizar el usuario
                sqlx::query(
                    r#"
                        UPDATE auth.users
                        SET 
                            username = $2,
                            email = $3,
                            password_hash = $4,
                            role = $5::auth.userrole,
                            storage_quota_bytes = $6,
                            storage_used_bytes = $7,
                            updated_at = $8,
                            last_login_at = $9,
                            active = $10
                        WHERE id = $1
                        "#,
                )
                .bind(user_clone.id())
                .bind(user_clone.username())
                .bind(user_clone.email())
                .bind(user_clone.password_hash())
                .bind(&user_clone.role().to_string())
                .bind(user_clone.storage_quota_bytes())
                .bind(user_clone.storage_used_bytes())
                .bind(user_clone.updated_at())
                .bind(user_clone.last_login_at())
                .bind(user_clone.is_active())
                .execute(&mut **tx)
                .await
                .map_err(Self::map_sqlx_error)?;

                // Podríamos realizar operaciones adicionales aquí dentro
                // de la misma transacción, como actualizar permisos, etc.

                Ok(user_clone)
            }) as BoxFuture<'_, UserRepositoryResult<User>>
        })
        .await?;

        Ok(user)
    }

    /// Actualiza solo el uso de almacenamiento de un usuario
    async fn update_storage_usage(
        &self,
        user_id: &str,
        usage_bytes: i64,
    ) -> UserRepositoryResult<()> {
        sqlx::query(
            r#"
            UPDATE auth.users
            SET 
                storage_used_bytes = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(usage_bytes)
        .execute(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    /// Actualiza la fecha de último inicio de sesión
    async fn update_last_login(&self, user_id: &str) -> UserRepositoryResult<()> {
        sqlx::query(
            r#"
            UPDATE auth.users
            SET 
                last_login_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    /// Lista usuarios con paginación
    async fn list_users(&self, limit: i64, offset: i64) -> UserRepositoryResult<Vec<User>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, username, email, password_hash, role::text as role_text, 
                storage_quota_bytes, storage_used_bytes, 
                created_at, updated_at, last_login_at, active
            FROM auth.users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        let users = rows
            .into_iter()
            .map(|row| {
                // Convert role string to UserRole enum for each row
                let role_str: Option<String> = row.try_get("role_text").unwrap_or(None);
                let role = match role_str.as_deref() {
                    Some("admin") => UserRole::Admin,
                    _ => UserRole::User,
                };

                User::from_data(
                    row.get("id"),
                    row.get("username"),
                    row.get("email"),
                    row.get("password_hash"),
                    role,
                    row.get("storage_quota_bytes"),
                    row.get("storage_used_bytes"),
                    row.get("created_at"),
                    row.get("updated_at"),
                    row.get("last_login_at"),
                    row.get("active"),
                )
            })
            .collect();

        Ok(users)
    }

    /// Activa o desactiva un usuario
    async fn set_user_active_status(
        &self,
        user_id: &str,
        active: bool,
    ) -> UserRepositoryResult<()> {
        sqlx::query(
            r#"
            UPDATE auth.users
            SET 
                active = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(active)
        .execute(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    /// Cambia la contraseña de un usuario
    async fn change_password(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> UserRepositoryResult<()> {
        sqlx::query(
            r#"
            UPDATE auth.users
            SET 
                password_hash = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(password_hash)
        .execute(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    /// Cambia el rol de un usuario
    async fn change_role(&self, user_id: &str, role: UserRole) -> UserRepositoryResult<()> {
        // Convertir el rol a string para el binding
        let role_str = role.to_string();

        sqlx::query(
            r#"
            UPDATE auth.users
            SET 
                role = $2::auth.userrole,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(&role_str)
        .execute(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    /// Lista usuarios por rol
    async fn list_users_by_role(&self, role: &str) -> UserRepositoryResult<Vec<User>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, username, email, password_hash, role::text as role_text, 
                storage_quota_bytes, storage_used_bytes, 
                created_at, updated_at, last_login_at, active
            FROM auth.users
            WHERE role::text = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(role)
        .fetch_all(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        let users = rows
            .into_iter()
            .map(|row| {
                // Convert role string to UserRole enum for each row
                let role_str: Option<String> = row.try_get("role_text").unwrap_or(None);
                let role = match role_str.as_deref() {
                    Some("admin") => UserRole::Admin,
                    _ => UserRole::User,
                };

                User::from_data(
                    row.get("id"),
                    row.get("username"),
                    row.get("email"),
                    row.get("password_hash"),
                    role,
                    row.get("storage_quota_bytes"),
                    row.get("storage_used_bytes"),
                    row.get("created_at"),
                    row.get("updated_at"),
                    row.get("last_login_at"),
                    row.get("active"),
                )
            })
            .collect();

        Ok(users)
    }

    /// Elimina un usuario
    async fn delete_user(&self, user_id: &str) -> UserRepositoryResult<()> {
        sqlx::query(
            r#"
            DELETE FROM auth.users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }
}

// Implementación del puerto de almacenamiento para la capa de aplicación
#[async_trait]
impl UserStoragePort for UserPgRepository {
    async fn create_user(&self, user: User) -> Result<User, DomainError> {
        UserRepository::create_user(self, user)
            .await
            .map_err(DomainError::from)
    }

    async fn get_user_by_id(&self, id: &str) -> Result<User, DomainError> {
        UserRepository::get_user_by_id(self, id)
            .await
            .map_err(DomainError::from)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User, DomainError> {
        UserRepository::get_user_by_username(self, username)
            .await
            .map_err(DomainError::from)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, DomainError> {
        UserRepository::get_user_by_email(self, email)
            .await
            .map_err(DomainError::from)
    }

    async fn update_user(&self, user: User) -> Result<User, DomainError> {
        UserRepository::update_user(self, user)
            .await
            .map_err(DomainError::from)
    }

    async fn update_storage_usage(
        &self,
        user_id: &str,
        usage_bytes: i64,
    ) -> Result<(), DomainError> {
        UserRepository::update_storage_usage(self, user_id, usage_bytes)
            .await
            .map_err(DomainError::from)
    }

    async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>, DomainError> {
        UserRepository::list_users(self, limit, offset)
            .await
            .map_err(DomainError::from)
    }

    async fn list_users_by_role(&self, role: &str) -> Result<Vec<User>, DomainError> {
        UserRepository::list_users_by_role(self, role)
            .await
            .map_err(DomainError::from)
    }

    async fn delete_user(&self, user_id: &str) -> Result<(), DomainError> {
        UserRepository::delete_user(self, user_id)
            .await
            .map_err(DomainError::from)
    }

    async fn change_password(&self, user_id: &str, password_hash: &str) -> Result<(), DomainError> {
        UserRepository::change_password(self, user_id, password_hash)
            .await
            .map_err(DomainError::from)
    }
}
