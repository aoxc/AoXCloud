use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use thiserror::Error;

use crate::application::ports::outbound::IdMappingPort;
use crate::domain::entities::folder::Folder;
use crate::domain::repositories::file_repository::FileRepositoryError;
use crate::domain::repositories::folder_repository::{FolderRepository, FolderRepositoryError};
use crate::domain::services::path_service::{PathService, StoragePath};

/// Errores específicos del mediador de almacenamiento
#[derive(Debug, Error)]
pub enum StorageMediatorError {
    #[error("Entidad no encontrada: {0}")]
    NotFound(String),

    #[error("Entidad ya existe: {0}")]
    AlreadyExists(String),

    #[error("Ruta inválida: {0}")]
    InvalidPath(String),

    #[error("Error de acceso: {0}")]
    AccessError(String),

    #[error("Error interno: {0}")]
    InternalError(String),

    #[error("Error de dominio: {0}")]
    DomainError(#[from] crate::common::errors::DomainError),
}

impl From<FolderRepositoryError> for StorageMediatorError {
    fn from(err: FolderRepositoryError) -> Self {
        match err {
            FolderRepositoryError::NotFound(id) => StorageMediatorError::NotFound(id),
            FolderRepositoryError::AlreadyExists(path) => StorageMediatorError::AlreadyExists(path),
            FolderRepositoryError::InvalidPath(path) => StorageMediatorError::InvalidPath(path),
            FolderRepositoryError::IoError(e) => StorageMediatorError::AccessError(e.to_string()),
            _ => StorageMediatorError::InternalError(err.to_string()),
        }
    }
}

impl From<FileRepositoryError> for StorageMediatorError {
    fn from(err: FileRepositoryError) -> Self {
        match err {
            FileRepositoryError::NotFound(id) => StorageMediatorError::NotFound(id),
            FileRepositoryError::AlreadyExists(path) => StorageMediatorError::AlreadyExists(path),
            FileRepositoryError::InvalidPath(path) => StorageMediatorError::InvalidPath(path),
            FileRepositoryError::IoError(e) => StorageMediatorError::AccessError(e.to_string()),
            _ => StorageMediatorError::InternalError(err.to_string()),
        }
    }
}

/// Tipo de resultado para las operaciones del mediador
pub type StorageMediatorResult<T> = Result<T, StorageMediatorError>;

/// Interfaz del servicio mediador entre repositorios de archivos y carpetas
#[async_trait]
pub trait StorageMediator: Send + Sync + 'static {
    /// Obtiene la ruta de una carpeta por su ID
    async fn get_folder_path(&self, folder_id: &str) -> StorageMediatorResult<PathBuf>;

    /// Obtiene la ruta de dominio de una carpeta por su ID
    async fn get_folder_storage_path(&self, folder_id: &str) -> StorageMediatorResult<StoragePath>;

    /// Obtiene todos los detalles de una carpeta por su ID
    async fn get_folder(&self, folder_id: &str) -> StorageMediatorResult<Folder>;

    /// Verifica si existe un archivo en una ruta específica
    async fn file_exists_at_path(&self, path: &Path) -> StorageMediatorResult<bool>;

    /// Verifica si existe un archivo en una ruta de dominio específica
    async fn file_exists_at_storage_path(
        &self,
        storage_path: &StoragePath,
    ) -> StorageMediatorResult<bool>;

    /// Verifica si existe una carpeta en una ruta específica
    async fn folder_exists_at_path(&self, path: &Path) -> StorageMediatorResult<bool>;

    /// Verifica si existe una carpeta en una ruta de dominio específica
    async fn folder_exists_at_storage_path(
        &self,
        storage_path: &StoragePath,
    ) -> StorageMediatorResult<bool>;

    /// Resuelve una ruta relativa a absoluta (legacy)
    fn resolve_path(&self, relative_path: &Path) -> PathBuf;

    /// Resuelve una ruta de dominio a una ruta física absoluta
    fn resolve_storage_path(&self, storage_path: &StoragePath) -> PathBuf;

    /// Crea un directorio si no existe (legacy)
    async fn ensure_directory(&self, path: &Path) -> StorageMediatorResult<()>;

    /// Crea un directorio si no existe
    async fn ensure_storage_directory(
        &self,
        storage_path: &StoragePath,
    ) -> StorageMediatorResult<()>;
}

/// Implementación concreta del mediador de almacenamiento
pub struct FileSystemStorageMediator {
    pub folder_repository: Arc<dyn FolderRepository>,
    pub path_service: Arc<PathService>,
    pub id_mapping: Arc<dyn IdMappingPort>,
}

impl FileSystemStorageMediator {
    pub fn new(
        folder_repository: Arc<dyn FolderRepository>,
        path_service: Arc<PathService>,
        id_mapping: Arc<dyn IdMappingPort>,
    ) -> Self {
        Self {
            folder_repository,
            path_service,
            id_mapping,
        }
    }

    /// Creates a stub implementation for initialization bootstrapping
    pub fn new_stub() -> StubStorageMediator {
        StubStorageMediator::new()
    }

    /// Overload para implementar inicialización diferida con repository placeholder
    pub fn new_with_lazy_folder(
        _folder_repository: Arc<RwLock<Option<Arc<dyn FolderRepository>>>>,
        path_service: Arc<PathService>,
        id_mapping: Arc<dyn IdMappingPort>,
    ) -> Self {
        // Create temporary stub repository
        let temp_repo = Arc::new(FolderRepositoryStub {});

        Self {
            folder_repository: temp_repo,
            path_service,
            id_mapping,
        }
    }
}

/// Stub repository for initialization
#[derive(Debug)]
pub struct FolderRepositoryStub {}

#[async_trait]
impl FolderRepository for FolderRepositoryStub {
    async fn create_folder(
        &self,
        _name: String,
        _parent_id: Option<String>,
    ) -> Result<Folder, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn get_folder_by_id(&self, _id: &str) -> Result<Folder, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn get_folder_by_storage_path(
        &self,
        _storage_path: &StoragePath,
    ) -> Result<Folder, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn list_folders(
        &self,
        _parent_id: Option<&str>,
    ) -> Result<Vec<Folder>, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn list_folders_paginated(
        &self,
        _parent_id: Option<&str>,
        _offset: usize,
        _limit: usize,
        _include_total: bool,
    ) -> Result<(Vec<Folder>, Option<usize>), FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn rename_folder(
        &self,
        _id: &str,
        _new_name: String,
    ) -> Result<Folder, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn move_folder(
        &self,
        _id: &str,
        _new_parent_id: Option<&str>,
    ) -> Result<Folder, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn delete_folder(&self, _id: &str) -> Result<(), FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    async fn folder_exists_at_storage_path(
        &self,
        _storage_path: &StoragePath,
    ) -> Result<bool, FolderRepositoryError> {
        Ok(false)
    }

    async fn get_folder_storage_path(
        &self,
        _id: &str,
    ) -> Result<StoragePath, FolderRepositoryError> {
        Ok(StoragePath::root())
    }

    // Legacy methods
    #[allow(deprecated)]
    async fn folder_exists(
        &self,
        _path: &std::path::PathBuf,
    ) -> Result<bool, FolderRepositoryError> {
        Ok(false)
    }

    #[allow(deprecated)]
    async fn get_folder_by_path(
        &self,
        _path: &std::path::PathBuf,
    ) -> Result<Folder, FolderRepositoryError> {
        Err(FolderRepositoryError::Other("Stub repository".to_string()))
    }

    // Trash functionality stubs
    async fn move_to_trash(&self, _folder_id: &str) -> Result<(), FolderRepositoryError> {
        Err(FolderRepositoryError::OperationNotSupported(
            "Trash feature temporarily disabled".to_string(),
        ))
    }

    async fn restore_from_trash(
        &self,
        _folder_id: &str,
        _original_path: &str,
    ) -> Result<(), FolderRepositoryError> {
        Err(FolderRepositoryError::OperationNotSupported(
            "Trash feature temporarily disabled".to_string(),
        ))
    }

    async fn delete_folder_permanently(
        &self,
        _folder_id: &str,
    ) -> Result<(), FolderRepositoryError> {
        Err(FolderRepositoryError::OperationNotSupported(
            "Trash feature temporarily disabled".to_string(),
        ))
    }
}

/// Stub implementation for initialization dependency issues
pub struct StubStorageMediator {
    #[allow(dead_code)]
    _path_service: Arc<PathService>,
}

impl StubStorageMediator {
    pub fn new() -> Self {
        let root_path = PathBuf::from("/tmp");
        let path_service = Arc::new(PathService::new(root_path));
        Self {
            _path_service: path_service,
        }
    }
}

#[async_trait]
impl StorageMediator for StubStorageMediator {
    async fn get_folder_path(&self, _folder_id: &str) -> StorageMediatorResult<PathBuf> {
        // Return a stub path
        Ok(PathBuf::from("/tmp"))
    }

    async fn get_folder_storage_path(
        &self,
        _folder_id: &str,
    ) -> StorageMediatorResult<StoragePath> {
        // Return a stub storage path
        Ok(StoragePath::root())
    }

    async fn get_folder(&self, _folder_id: &str) -> StorageMediatorResult<Folder> {
        // This is a stub that should never be called during initialization
        Err(StorageMediatorError::NotFound(
            "Stub not implemented".to_string(),
        ))
    }

    async fn file_exists_at_path(&self, _path: &Path) -> StorageMediatorResult<bool> {
        Ok(false)
    }

    async fn file_exists_at_storage_path(
        &self,
        _storage_path: &StoragePath,
    ) -> StorageMediatorResult<bool> {
        Ok(false)
    }

    async fn folder_exists_at_path(&self, _path: &Path) -> StorageMediatorResult<bool> {
        Ok(false)
    }

    async fn folder_exists_at_storage_path(
        &self,
        _storage_path: &StoragePath,
    ) -> StorageMediatorResult<bool> {
        Ok(false)
    }

    fn resolve_path(&self, _relative_path: &Path) -> PathBuf {
        PathBuf::from("/tmp")
    }

    fn resolve_storage_path(&self, _storage_path: &StoragePath) -> PathBuf {
        PathBuf::from("/tmp")
    }

    async fn ensure_directory(&self, _path: &Path) -> StorageMediatorResult<()> {
        Ok(())
    }

    async fn ensure_storage_directory(
        &self,
        _storage_path: &StoragePath,
    ) -> StorageMediatorResult<()> {
        Ok(())
    }
}

#[async_trait]
impl StorageMediator for FileSystemStorageMediator {
    async fn get_folder_path(&self, folder_id: &str) -> StorageMediatorResult<PathBuf> {
        let folder = self
            .folder_repository
            .get_folder_by_id(folder_id)
            .await
            .map_err(StorageMediatorError::from)?;

        // Need to get the path from folder ID
        let storage_path = self
            .id_mapping
            .get_path_by_id(folder.id())
            .await
            .map_err(StorageMediatorError::from)?;

        // Convert StoragePath to PathBuf
        let path_buf = self.path_service.resolve_path(&storage_path);
        Ok(path_buf)
    }

    async fn get_folder_storage_path(&self, folder_id: &str) -> StorageMediatorResult<StoragePath> {
        let folder = self
            .folder_repository
            .get_folder_by_id(folder_id)
            .await
            .map_err(StorageMediatorError::from)?;

        // Get path by folder ID - will already be a StoragePath
        let storage_path = self
            .id_mapping
            .get_path_by_id(folder.id())
            .await
            .map_err(StorageMediatorError::from)?;

        Ok(storage_path)
    }

    async fn get_folder(&self, folder_id: &str) -> StorageMediatorResult<Folder> {
        let folder = self
            .folder_repository
            .get_folder_by_id(folder_id)
            .await
            .map_err(StorageMediatorError::from)?;

        Ok(folder)
    }

    async fn file_exists_at_path(&self, path: &Path) -> StorageMediatorResult<bool> {
        let abs_path = self.resolve_path(path);

        // Verificar si existe como archivo (no como directorio)
        let exists = abs_path.exists() && abs_path.is_file();

        Ok(exists)
    }

    async fn file_exists_at_storage_path(
        &self,
        storage_path: &StoragePath,
    ) -> StorageMediatorResult<bool> {
        let abs_path = self.resolve_storage_path(storage_path);

        // Verificar si existe como archivo (no como directorio)
        let exists = abs_path.exists() && abs_path.is_file();

        Ok(exists)
    }

    async fn folder_exists_at_path(&self, path: &Path) -> StorageMediatorResult<bool> {
        let abs_path = self.resolve_path(path);

        // Verificar si existe como directorio
        let exists = abs_path.exists() && abs_path.is_dir();

        Ok(exists)
    }

    async fn folder_exists_at_storage_path(
        &self,
        storage_path: &StoragePath,
    ) -> StorageMediatorResult<bool> {
        let abs_path = self.resolve_storage_path(storage_path);

        // Verificar si existe como directorio
        let exists = abs_path.exists() && abs_path.is_dir();

        Ok(exists)
    }

    fn resolve_path(&self, relative_path: &Path) -> PathBuf {
        // Legacy method using PathBuf
        let path_str = relative_path.to_string_lossy().to_string();
        let storage_path = StoragePath::from_string(&path_str);
        self.path_service.resolve_path(&storage_path)
    }

    fn resolve_storage_path(&self, storage_path: &StoragePath) -> PathBuf {
        self.path_service.resolve_path(storage_path)
    }

    async fn ensure_directory(&self, path: &Path) -> StorageMediatorResult<()> {
        let abs_path = self.resolve_path(path);

        // Crear directorios si no existen
        if !abs_path.exists() {
            tokio::fs::create_dir_all(&abs_path).await.map_err(|e| {
                StorageMediatorError::AccessError(format!("No se pudo crear el directorio: {}", e))
            })?;
        } else if !abs_path.is_dir() {
            return Err(StorageMediatorError::InvalidPath(format!(
                "La ruta existe pero no es un directorio: {}",
                abs_path.display()
            )));
        }

        Ok(())
    }

    async fn ensure_storage_directory(
        &self,
        storage_path: &StoragePath,
    ) -> StorageMediatorResult<()> {
        let abs_path = self.resolve_storage_path(storage_path);

        // Crear directorios si no existen
        if !abs_path.exists() {
            tokio::fs::create_dir_all(&abs_path).await.map_err(|e| {
                StorageMediatorError::AccessError(format!("No se pudo crear el directorio: {}", e))
            })?;
        } else if !abs_path.is_dir() {
            return Err(StorageMediatorError::InvalidPath(format!(
                "La ruta existe pero no es un directorio: {}",
                abs_path.display()
            )));
        }

        Ok(())
    }
}
