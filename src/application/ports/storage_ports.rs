use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use serde_json::Value;
use std::path::PathBuf;

use crate::common::errors::DomainError;
use crate::domain::entities::file::File;
use crate::domain::services::path_service::StoragePath;

/// Puerto secundario para lectura de archivos
#[async_trait]
pub trait FileReadPort: Send + Sync + 'static {
    /// Obtiene un archivo por su ID
    async fn get_file(&self, id: &str) -> Result<File, DomainError>;

    /// Lista archivos en una carpeta
    async fn list_files(&self, folder_id: Option<&str>) -> Result<Vec<File>, DomainError>;

    /// Obtiene contenido de archivo como bytes
    async fn get_file_content(&self, id: &str) -> Result<Vec<u8>, DomainError>;

    /// Obtiene contenido de archivo como stream
    async fn get_file_stream(
        &self,
        id: &str,
    ) -> Result<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>, DomainError>;
}

/// Puerto secundario para escritura de archivos
#[async_trait]
pub trait FileWritePort: Send + Sync + 'static {
    /// Guarda un nuevo archivo desde bytes
    async fn save_file(
        &self,
        name: String,
        folder_id: Option<String>,
        content_type: String,
        content: Vec<u8>,
    ) -> Result<File, DomainError>;

    /// Mueve un archivo a otra carpeta
    async fn move_file(
        &self,
        file_id: &str,
        target_folder_id: Option<String>,
    ) -> Result<File, DomainError>;

    /// Elimina un archivo
    async fn delete_file(&self, id: &str) -> Result<(), DomainError>;

    /// Obtiene detalles de una carpeta
    async fn get_folder_details(&self, folder_id: &str) -> Result<File, DomainError>;

    /// Obtiene la ruta de una carpeta como string
    async fn get_folder_path_str(&self, folder_id: &str) -> Result<String, DomainError>;
}

/// Puerto secundario para resolución de rutas de archivos
#[async_trait]
pub trait FilePathResolutionPort: Send + Sync + 'static {
    /// Obtiene la ruta de almacenamiento de un archivo
    async fn get_file_path(&self, id: &str) -> Result<StoragePath, DomainError>;

    /// Resuelve una ruta de dominio a una ruta física
    fn resolve_path(&self, storage_path: &StoragePath) -> PathBuf;
}

/// Puerto secundario para verificación de existencia de archivos/directorios
#[async_trait]
pub trait StorageVerificationPort: Send + Sync + 'static {
    /// Verifica si existe un archivo en la ruta dada
    async fn file_exists(&self, storage_path: &StoragePath) -> Result<bool, DomainError>;

    /// Verifica si existe un directorio en la ruta dada
    async fn directory_exists(&self, storage_path: &StoragePath) -> Result<bool, DomainError>;
}

/// Puerto secundario para gestión de directorios
#[async_trait]
pub trait DirectoryManagementPort: Send + Sync + 'static {
    /// Crea directorios si no existen
    async fn ensure_directory(&self, storage_path: &StoragePath) -> Result<(), DomainError>;
}

/// Puerto secundario para gestión de uso de almacenamiento
#[async_trait]
pub trait StorageUsagePort: Send + Sync + 'static {
    /// Actualiza estadísticas de uso de almacenamiento para un usuario
    async fn update_user_storage_usage(&self, user_id: &str) -> Result<i64, DomainError>;

    /// Actualiza estadísticas de uso de almacenamiento para todos los usuarios
    async fn update_all_users_storage_usage(&self) -> Result<(), DomainError>;
}

/// Generic storage service interface for calendar and contact services
#[async_trait]
pub trait StorageUseCase: Send + Sync + 'static {
    /// Handle a request with the specified action and parameters
    async fn handle_request(&self, action: &str, params: Value) -> Result<Value, DomainError>;
}
