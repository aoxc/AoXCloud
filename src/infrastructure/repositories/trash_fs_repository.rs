use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::application::ports::outbound::IdMappingPort;
use crate::common::errors::{DomainError, ErrorKind, Result};
use crate::domain::entities::trashed_item::{TrashedItem, TrashedItemType};
use crate::domain::repositories::trash_repository::TrashRepository;

/// Estructura para almacenar elementos en la papelera en formato JSON
#[derive(Debug, Serialize, Deserialize)]
struct TrashedItemEntry {
    id: String,
    original_id: String,
    user_id: String,
    item_type: String,
    name: String,
    original_path: String,
    trashed_at: String,
    deletion_date: String,
}

/// Implementación del repositorio de papelera usando el sistema de archivos
pub struct TrashFsRepository {
    trash_dir: PathBuf,
    trash_index_path: PathBuf,
    id_mapping_service: Arc<dyn IdMappingPort>,
}

impl TrashFsRepository {
    pub fn new(storage_root: impl AsRef<Path>, id_mapping_service: Arc<dyn IdMappingPort>) -> Self {
        let trash_dir = storage_root.as_ref().join(".trash");
        let trash_index_path = trash_dir.join("trash_index.json");

        Self {
            trash_dir,
            trash_index_path,
            id_mapping_service,
        }
    }

    /// Asegura que existe el directorio de papelera
    async fn ensure_trash_dir(&self) -> Result<()> {
        debug!(
            "Checking if trash directory exists: {}",
            self.trash_dir.display()
        );
        if !self.trash_dir.exists() {
            debug!(
                "Trash directory does not exist, creating it: {}",
                self.trash_dir.display()
            );
            fs::create_dir_all(&self.trash_dir).await.map_err(|e| {
                error!(
                    "Failed to create trash directory {}: {}",
                    self.trash_dir.display(),
                    e
                );
                DomainError::new(
                    ErrorKind::InternalError,
                    "Trash",
                    format!(
                        "Failed to create trash directory {}: {}",
                        self.trash_dir.display(),
                        e
                    ),
                )
            })?;
            debug!("Trash directory created successfully");
        } else {
            debug!("Trash directory already exists");
        }

        // Ensure the files directory exists
        let files_dir = self.trash_dir.join("files");
        debug!(
            "Checking if trash files directory exists: {}",
            files_dir.display()
        );
        if !files_dir.exists() {
            debug!(
                "Trash files directory does not exist, creating it: {}",
                files_dir.display()
            );
            fs::create_dir_all(&files_dir).await.map_err(|e| {
                error!(
                    "Failed to create trash files directory {}: {}",
                    files_dir.display(),
                    e
                );
                DomainError::new(
                    ErrorKind::InternalError,
                    "Trash",
                    format!(
                        "Failed to create trash files directory {}: {}",
                        files_dir.display(),
                        e
                    ),
                )
            })?;
            debug!("Trash files directory created successfully");
        } else {
            debug!("Trash files directory already exists");
        }

        // Also ensure the folders directory exists
        let folders_dir = self.trash_dir.join("folders");
        debug!(
            "Checking if trash folders directory exists: {}",
            folders_dir.display()
        );
        if !folders_dir.exists() {
            debug!(
                "Trash folders directory does not exist, creating it: {}",
                folders_dir.display()
            );
            fs::create_dir_all(&folders_dir).await.map_err(|e| {
                error!(
                    "Failed to create trash folders directory {}: {}",
                    folders_dir.display(),
                    e
                );
                DomainError::new(
                    ErrorKind::InternalError,
                    "Trash",
                    format!(
                        "Failed to create trash folders directory {}: {}",
                        folders_dir.display(),
                        e
                    ),
                )
            })?;
            debug!("Trash folders directory created successfully");
        } else {
            debug!("Trash folders directory already exists");
        }

        Ok(())
    }

    /// Obtiene todas las entradas del índice de papelera
    async fn get_trash_entries(&self) -> Result<Vec<TrashedItemEntry>> {
        self.ensure_trash_dir().await?;

        if !self.trash_index_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.trash_index_path)
            .await
            .map_err(|e| {
                DomainError::new(
                    ErrorKind::InternalError,
                    "Trash",
                    format!("Failed to read trash index: {}", e),
                )
            })?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let entries: Vec<TrashedItemEntry> = serde_json::from_str(&content).map_err(|e| {
            DomainError::new(
                ErrorKind::InternalError,
                "Trash",
                format!("Failed to parse trash index: {}", e),
            )
        })?;

        Ok(entries)
    }

    /// Guarda todas las entradas en el índice de papelera
    async fn save_trash_entries(&self, entries: Vec<TrashedItemEntry>) -> Result<()> {
        self.ensure_trash_dir().await?;

        let json = serde_json::to_string_pretty(&entries).map_err(|e| {
            DomainError::new(
                ErrorKind::InternalError,
                "Trash",
                format!("Failed to serialize trash index: {}", e),
            )
        })?;

        fs::write(&self.trash_index_path, json).await.map_err(|e| {
            DomainError::new(
                ErrorKind::InternalError,
                "Trash",
                format!("Failed to write trash index: {}", e),
            )
        })?;

        Ok(())
    }

    /// Convierte una entrada JSON a entidad TrashedItem
    fn entry_to_trashed_item(&self, entry: TrashedItemEntry) -> Result<TrashedItem> {
        let item_type = match entry.item_type.as_str() {
            "file" => TrashedItemType::File,
            "folder" => TrashedItemType::Folder,
            _ => {
                return Err(DomainError::new(
                    ErrorKind::InvalidInput,
                    "Trash",
                    format!("Invalid trashed item type: {}", entry.item_type),
                ))
            }
        };

        let original_id = Uuid::parse_str(&entry.original_id).map_err(|e| {
            DomainError::validation_error(format!("Invalid original ID format: {}", e))
        })?;

        let id = Uuid::parse_str(&entry.id)
            .map_err(|e| DomainError::validation_error(format!("Invalid ID format: {}", e)))?;

        let user_id = Uuid::parse_str(&entry.user_id)
            .map_err(|e| DomainError::validation_error(format!("Invalid user ID format: {}", e)))?;

        let trashed_at = chrono::DateTime::parse_from_rfc3339(&entry.trashed_at)
            .map_err(|e| DomainError::validation_error(format!("Invalid trashed_at date: {}", e)))?
            .with_timezone(&Utc);

        let deletion_date = chrono::DateTime::parse_from_rfc3339(&entry.deletion_date)
            .map_err(|e| DomainError::validation_error(format!("Invalid deletion_date: {}", e)))?
            .with_timezone(&Utc);

        Ok(TrashedItem {
            id,
            original_id,
            user_id,
            item_type,
            name: entry.name,
            original_path: entry.original_path,
            trashed_at,
            deletion_date,
        })
    }

    /// Convierte una entidad TrashedItem a entrada JSON
    fn trashed_item_to_entry(&self, item: &TrashedItem) -> TrashedItemEntry {
        TrashedItemEntry {
            id: item.id.to_string(),
            original_id: item.original_id.to_string(),
            user_id: item.user_id.to_string(),
            item_type: match item.item_type {
                TrashedItemType::File => "file".to_string(),
                TrashedItemType::Folder => "folder".to_string(),
            },
            name: item.name.clone(),
            original_path: item.original_path.clone(),
            trashed_at: item.trashed_at.to_rfc3339(),
            deletion_date: item.deletion_date.to_rfc3339(),
        }
    }

    /// Obtiene la ruta de un elemento en la papelera
    fn get_trash_path_for_item(&self, user_id: &Uuid, item_id: &Uuid) -> PathBuf {
        self.trash_dir
            .join("files")
            .join(user_id.to_string())
            .join(item_id.to_string())
    }
}

#[async_trait]
impl TrashRepository for TrashFsRepository {
    #[instrument(skip(self))]
    async fn add_to_trash(&self, item: &TrashedItem) -> Result<()> {
        debug!(
            "Añadiendo elemento a la papelera: id={}, user={}",
            item.id, item.user_id
        );

        // Aseguramos que existe el directorio de la papelera para este usuario
        let user_trash_dir = self.trash_dir.join("files").join(item.user_id.to_string());
        debug!("User trash directory path: {}", user_trash_dir.display());

        // Create the user-specific trash directory
        debug!(
            "Creating user trash directory: {}",
            user_trash_dir.display()
        );
        match fs::create_dir_all(&user_trash_dir).await {
            Ok(_) => debug!("User trash directory created successfully"),
            Err(e) => {
                error!(
                    "Failed to create user trash directory {}: {}",
                    user_trash_dir.display(),
                    e
                );
                return Err(DomainError::new(
                    ErrorKind::InternalError,
                    "Trash",
                    format!("Failed to create user trash directory: {}", e),
                ));
            }
        }

        // Log the current trash entries before adding the new one
        let mut entries = self.get_trash_entries().await?;
        debug!("Current trash entries count: {}", entries.len());

        // Create the entry for the trash index
        let entry = self.trashed_item_to_entry(item);
        debug!(
            "Created trash entry: id={}, original_id={}, name={}",
            entry.id, entry.original_id, entry.name
        );

        // Add the entry to the index and save
        entries.push(entry);
        debug!("Saving updated trash index with {} entries", entries.len());
        self.save_trash_entries(entries).await?;
        debug!("Trash index updated successfully");

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_trash_items(&self, user_id: &Uuid) -> Result<Vec<TrashedItem>> {
        debug!("Obteniendo elementos en papelera para usuario: {}", user_id);

        let entries = self.get_trash_entries().await?;

        let user_id_str = user_id.to_string();
        let user_entries = entries
            .into_iter()
            .filter(|entry| entry.user_id == user_id_str)
            .collect::<Vec<_>>();

        let mut items = Vec::new();
        for entry in user_entries {
            match self.entry_to_trashed_item(entry) {
                Ok(item) => items.push(item),
                Err(e) => error!("Error converting trash entry to item: {}", e),
            }
        }

        Ok(items)
    }

    #[instrument(skip(self))]
    async fn get_trash_item(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<TrashedItem>> {
        debug!("Buscando elemento en papelera: id={}, user={}", id, user_id);

        let entries = self.get_trash_entries().await?;

        let id_str = id.to_string();
        let user_id_str = user_id.to_string();

        let item_entry = entries
            .into_iter()
            .find(|entry| entry.id == id_str && entry.user_id == user_id_str);

        match item_entry {
            Some(entry) => {
                let item = self.entry_to_trashed_item(entry)?;
                Ok(Some(item))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn restore_from_trash(&self, id: &Uuid, user_id: &Uuid) -> Result<()> {
        debug!(
            "Restaurando elemento de la papelera: id={}, user={}",
            id, user_id
        );

        let mut entries = self.get_trash_entries().await?;

        let id_str = id.to_string();
        let user_id_str = user_id.to_string();

        let index = entries
            .iter()
            .position(|entry| entry.id == id_str && entry.user_id == user_id_str);

        if let Some(index) = index {
            entries.remove(index);
            self.save_trash_entries(entries).await?;
            Ok(())
        } else {
            Err(DomainError::not_found("TrashedItem", id.to_string()))
        }
    }

    #[instrument(skip(self))]
    async fn delete_permanently(&self, id: &Uuid, user_id: &Uuid) -> Result<()> {
        debug!(
            "Eliminando permanentemente elemento de la papelera: id={}, user={}",
            id, user_id
        );

        // Simplemente eliminamos la entrada del índice
        // Los archivos físicos se eliminarán a través del repositorio correspondiente
        self.restore_from_trash(id, user_id).await
    }

    #[instrument(skip(self))]
    async fn clear_trash(&self, user_id: &Uuid) -> Result<()> {
        debug!("Limpiando papelera para usuario: {}", user_id);

        let mut entries = self.get_trash_entries().await?;
        let user_id_str = user_id.to_string();

        entries.retain(|entry| entry.user_id != user_id_str);
        self.save_trash_entries(entries).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_expired_items(&self) -> Result<Vec<TrashedItem>> {
        debug!("Buscando elementos de papelera expirados");

        let entries = self.get_trash_entries().await?;
        let now = Utc::now();

        let mut expired_items = Vec::new();

        for entry in entries {
            match chrono::DateTime::parse_from_rfc3339(&entry.deletion_date) {
                Ok(date) => {
                    let utc_date = date.with_timezone(&Utc);
                    if utc_date <= now {
                        match self.entry_to_trashed_item(entry) {
                            Ok(item) => expired_items.push(item),
                            Err(e) => error!("Error converting expired trash entry: {}", e),
                        }
                    }
                }
                Err(e) => error!("Invalid date format in trash entry: {}", e),
            }
        }

        Ok(expired_items)
    }
}
