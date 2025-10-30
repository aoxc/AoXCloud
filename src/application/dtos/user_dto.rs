use crate::domain::entities::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub storage_quota_bytes: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub active: bool,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id().to_string(),
            username: user.username().to_string(),
            email: user.email().to_string(),
            role: format!("{}", user.role()),
            storage_quota_bytes: user.storage_quota_bytes(),
            storage_used_bytes: user.storage_used_bytes(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
            last_login_at: user.last_login_at(),
            active: user.is_active(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginDto {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisterDto {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponseDto {
    pub user: UserDto,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordDto {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenDto {
    pub refresh_token: String,
}
