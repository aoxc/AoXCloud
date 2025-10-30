use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

use crate::application::services::auth_application_service::AuthApplicationService;
use crate::application::services::folder_service::FolderService;
use crate::common::config::AppConfig;
use crate::common::di::AuthServices;
use crate::domain::services::auth_service::AuthService;
use crate::infrastructure::repositories::{SessionPgRepository, UserPgRepository};

pub async fn create_auth_services(
    config: &AppConfig,
    pool: Arc<PgPool>,
    folder_service: Option<Arc<FolderService>>,
) -> Result<AuthServices> {
    // Crear servicio de dominio de autenticación
    let auth_service = Arc::new(AuthService::new(
        config.auth.jwt_secret.clone(),
        config.auth.access_token_expiry_secs,
        config.auth.refresh_token_expiry_secs,
    ));

    // Crear repositorios PostgreSQL
    let user_repository = Arc::new(UserPgRepository::new(pool.clone()));
    let session_repository = Arc::new(SessionPgRepository::new(pool.clone()));

    // Crear servicio de aplicación de autenticación
    let mut auth_app_service =
        AuthApplicationService::new(user_repository, session_repository, auth_service.clone());

    // Configurar servicio de carpetas si está disponible
    if let Some(folder_svc) = folder_service {
        auth_app_service = auth_app_service.with_folder_service(folder_svc);
    }

    // Empaquetar servicio en Arc
    let auth_application_service = Arc::new(auth_app_service);

    Ok(AuthServices {
        auth_service,
        auth_application_service,
    })
}
