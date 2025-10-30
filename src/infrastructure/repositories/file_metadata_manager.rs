use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::time;

use crate::common::config::AppConfig;
use crate::common::errors::DomainError;
use crate::domain::entities::file::File;
use crate::domain::services::path_service::StoragePath;
use crate::infrastructure::services::file_metadata_cache::{
    CacheEntryType, FileMetadata, FileMetadataCache,
};

/// Gestor de metadatos de archivos que encapsula la lógica de caché
pub struct FileMetadataManager {
    metadata_cache: Arc<FileMetadataCache>,
    config: AppConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("Error de E/S al acceder a los metadatos: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Timeout al acceder a los metadatos: {0}")]
    Timeout(String),

    #[error("Metadatos no disponibles: {0}")]
    Unavailable(String),
}

impl From<MetadataError> for DomainError {
    fn from(err: MetadataError) -> Self {
        match err {
            MetadataError::IoError(e) => DomainError::internal_error("FileMetadata", e.to_string()),
            MetadataError::Timeout(msg) => DomainError::internal_error("FileMetadata", msg),
            MetadataError::Unavailable(msg) => DomainError::not_found("FileMetadata", msg),
        }
    }
}

impl FileMetadataManager {
    /// Crea un nuevo gestor de metadatos
    pub fn new(metadata_cache: Arc<FileMetadataCache>, config: AppConfig) -> Self {
        Self {
            metadata_cache,
            config,
        }
    }

    /// Crea un gestor por defecto para pruebas
    pub fn default() -> Self {
        Self {
            metadata_cache: Arc::new(FileMetadataCache::default()),
            config: AppConfig::default(),
        }
    }

    /// Comprueba si un archivo existe en la ruta especificada con caché
    pub async fn file_exists(&self, abs_path: &PathBuf) -> Result<bool, MetadataError> {
        // Intentar obtener del caché avanzado primero
        if let Some(is_file) = self.metadata_cache.is_file(&abs_path).await {
            tracing::debug!(
                "Metadata cache hit for existence check: {} - path: {}",
                is_file,
                abs_path.display()
            );
            return Ok(is_file);
        }

        // Si no está en caché, verificar directamente y actualizar caché
        tracing::debug!(
            "Metadata cache miss for existence check: {}",
            abs_path.display()
        );

        // Utilizar timeout para evitar bloqueo
        match time::timeout(self.config.timeouts.file_timeout(), fs::metadata(&abs_path)).await {
            Ok(Ok(metadata)) => {
                let is_file = metadata.is_file();

                // Actualizar la caché con información fresca
                if let Err(e) = self.metadata_cache.refresh_metadata(&abs_path).await {
                    tracing::warn!("Failed to update cache for {}: {}", abs_path.display(), e);
                }

                if is_file {
                    tracing::debug!("File exists and is accessible: {}", abs_path.display());
                    Ok(true)
                } else {
                    tracing::warn!("Path exists but is not a file: {}", abs_path.display());
                    Ok(false)
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("File check failed: {} - {}", abs_path.display(), e);

                // Añadir a caché como no existente
                let entry_type = CacheEntryType::Unknown;
                let file_metadata = FileMetadata::new(
                    abs_path.clone(),
                    false,
                    entry_type,
                    None,
                    None,
                    None,
                    None,
                    Duration::from_millis(self.config.timeouts.file_operation_ms),
                );
                self.metadata_cache.update_cache(file_metadata).await;

                Ok(false)
            }
            Err(_) => {
                tracing::warn!("Timeout checking file metadata: {}", abs_path.display());
                Err(MetadataError::Timeout(format!(
                    "Timeout checking file: {}",
                    abs_path.display()
                )))
            }
        }
    }

    /// Obtiene metadatos de archivo (tamaño, fechas creación/modificación) con caché
    pub async fn get_file_metadata(
        &self,
        abs_path: &PathBuf,
    ) -> Result<(u64, u64, u64), MetadataError> {
        // Intentar obtener de caché primero
        if let Some(cached_metadata) = self.metadata_cache.get_metadata(abs_path).await {
            if let (Some(size), Some(created_at), Some(modified_at)) = (
                cached_metadata.size,
                cached_metadata.created_at,
                cached_metadata.modified_at,
            ) {
                tracing::debug!("Using cached metadata for: {}", abs_path.display());
                return Ok((size, created_at, modified_at));
            }
        }

        // Si no está en caché o metadatos incompletos, cargar desde sistema de archivos
        let metadata =
            match time::timeout(self.config.timeouts.file_timeout(), fs::metadata(&abs_path)).await
            {
                Ok(Ok(metadata)) => metadata,
                Ok(Err(e)) => return Err(MetadataError::IoError(e)),
                Err(_) => {
                    return Err(MetadataError::Timeout(format!(
                        "Timeout getting metadata for: {}",
                        abs_path.display()
                    )))
                }
            };

        let size = metadata.len();

        // Get creation timestamp
        let created_at = metadata
            .created()
            .map(|time| {
                time.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or_else(|_| 0);

        // Get modification timestamp
        let modified_at = metadata
            .modified()
            .map(|time| {
                time.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or_else(|_| 0);

        // Actualizar caché si es posible
        if let Err(e) = self.metadata_cache.refresh_metadata(abs_path).await {
            tracing::warn!(
                "Failed to update metadata cache for {}: {}",
                abs_path.display(),
                e
            );
        }

        Ok((size, created_at, modified_at))
    }

    /// Invalida la entrada de caché para un archivo
    pub async fn invalidate(&self, abs_path: &PathBuf) {
        self.metadata_cache.invalidate(abs_path).await;
    }

    /// Invalida la entrada de caché para un directorio y su contenido
    pub async fn invalidate_directory(&self, dir_path: &PathBuf) {
        self.metadata_cache.invalidate_directory(dir_path).await;
    }

    /// Actualiza los metadatos de un archivo en la caché
    pub async fn update_file_metadata(
        &self,
        file: &crate::domain::entities::file::File,
    ) -> Result<(), MetadataError> {
        // Crear una ruta absoluta para el archivo
        let abs_path = PathBuf::from(format!(
            "{}/{}",
            self.config.storage_path.display(),
            file.storage_path().to_string()
        ));

        // Crear un objeto FileMetadata
        let metadata = FileMetadataCache::create_metadata_from_file(file, abs_path.clone());

        // Actualizar la caché
        self.metadata_cache.update_cache(metadata).await;

        Ok(())
    }

    /// Obtiene información de una carpeta por ID
    pub async fn get_folder_by_id(&self, folder_id: &str) -> Result<File, MetadataError> {
        // Implementación simplificada que solo busca en la caché de metadatos
        // pero que devuelve una estructura mínima para el servicio de uso de almacenamiento

        // En una implementación real, se consultaría un índice persistente
        // Para esta implementación básica, usaremos un método simplificado

        // Crear un objeto StoragePath mínimo
        let storage_path = StoragePath::from_string(&format!("/{}", folder_id));

        // Creamos una carpeta con información mínima
        // Esta implementación es un placeholder - en una situación real
        // consultaríamos el mapa folder_id -> folder_metadata en el sistema
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Verificamos si el nombre contiene información del usuario
        let folder_name = if folder_id.contains('-') {
            // Asumimos un formato UUID v4, intentamos usar "Mi Carpeta - username" como nombre
            format!("Mi Carpeta - usuario")
        } else {
            // Si no, usamos el ID como nombre
            folder_id.to_string()
        };

        let folder = File::new_folder(
            folder_id.to_string(),
            folder_name,
            storage_path,
            None, // parent_id
            now,  // created_at
            now,  // updated_at
        )
        .map_err(|e| MetadataError::Unavailable(format!("Error creating folder entity: {}", e)))?;

        Ok(folder)
    }
}
