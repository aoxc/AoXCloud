use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use crate::application::services::auth_application_service::AuthApplicationService;
use crate::domain::services::auth_service::AuthService;

use crate::application::ports::favorites_ports::FavoritesUseCase;
use crate::application::ports::file_ports::{
    FileManagementUseCase, FileRetrievalUseCase, FileUploadUseCase, FileUseCaseFactory,
};
use crate::application::ports::inbound::{FileUseCase, FolderUseCase, SearchUseCase};
use crate::application::ports::outbound::{FileStoragePort, FolderStoragePort};
use crate::application::ports::recent_ports::RecentItemsUseCase;
use crate::application::ports::storage_ports::{FileReadPort, FileWritePort};
use crate::application::ports::trash_ports::TrashUseCase;
use crate::application::services::file_service::FileService;
use crate::application::services::folder_service::FolderService;
use crate::application::services::i18n_application_service::I18nApplicationService;
use crate::application::services::storage_mediator::{FileSystemStorageMediator, StorageMediator};
use crate::application::services::{
    AppFileUseCaseFactory, FileManagementService, FileRetrievalService, FileUploadService,
};
use crate::common::config::AppConfig;
use crate::common::errors::DomainError;
use crate::domain::services::i18n_service::I18nService;
use crate::domain::services::path_service::PathService;
use crate::infrastructure::repositories::file_fs_repository::FileFsRepository;
use crate::infrastructure::repositories::folder_fs_repository::FolderFsRepository;
use crate::infrastructure::repositories::trash_fs_repository::TrashFsRepository;
use crate::infrastructure::repositories::{
    FileFsReadRepository, FileFsWriteRepository, FileMetadataManager, FilePathResolver,
};
use crate::infrastructure::services::cache_manager::StorageCacheManager;
use crate::infrastructure::services::file_metadata_cache::FileMetadataCache;
use crate::infrastructure::services::file_system_i18n_service::FileSystemI18nService;
use crate::infrastructure::services::id_mapping_service::IdMappingService;

/// Fábrica para los diferentes componentes de la aplicación
#[allow(dead_code)]
pub struct AppServiceFactory {
    storage_path: PathBuf,
    locales_path: PathBuf,
    config: AppConfig,
}

impl AppServiceFactory {
    /// Crea una nueva fábrica de servicios
    #[allow(dead_code)]
    pub fn new(storage_path: PathBuf, locales_path: PathBuf) -> Self {
        Self {
            storage_path,
            locales_path,
            config: AppConfig::default(),
        }
    }

    /// Crea una nueva fábrica de servicios con configuración personalizada
    #[allow(dead_code)]
    pub fn with_config(storage_path: PathBuf, locales_path: PathBuf, config: AppConfig) -> Self {
        Self {
            storage_path,
            locales_path,
            config,
        }
    }

    /// Inicializa los servicios base del sistema
    #[allow(dead_code)]
    pub async fn create_core_services(&self) -> Result<CoreServices, DomainError> {
        // Path service
        let path_service = Arc::new(PathService::new(self.storage_path.clone()));

        // Cache manager
        // TTL values in milliseconds and max entries for cache
        let file_ttl_ms = 60_000; // 1 minute for files
        let dir_ttl_ms = 120_000; // 2 minutes for directories
        let max_entries = 10_000; // Maximum cache entries
        let cache_manager = Arc::new(StorageCacheManager::new(
            file_ttl_ms,
            dir_ttl_ms,
            max_entries,
        ));

        // Iniciar tarea de limpieza de caché en segundo plano
        let cache_manager_clone = cache_manager.clone();
        tokio::spawn(async move {
            StorageCacheManager::start_cleanup_task(cache_manager_clone).await;
        });

        // ID mapping service
        let id_mapping_path = self.storage_path.join("folder_ids.json");
        let id_mapping_service = Arc::new(IdMappingService::new(id_mapping_path).await?);

        Ok(CoreServices {
            path_service,
            cache_manager,
            id_mapping_service,
            config: self.config.clone(),
        })
    }

    /// Inicializa los servicios de repositorio utilizando el patrón Builder mejorado
    #[allow(dead_code)]
    pub fn create_repository_services(&self, core: &CoreServices) -> RepositoryServices {
        // Storage mediator - con inicialización diferida para folder repository
        let folder_repository_holder = Arc::new(RwLock::new(None));

        let storage_mediator = Arc::new(FileSystemStorageMediator::new_with_lazy_folder(
            folder_repository_holder.clone(),
            core.path_service.clone(),
            core.id_mapping_service.clone(),
        ));

        // Folder repository
        let folder_repository = Arc::new(FolderFsRepository::new(
            self.storage_path.clone(),
            storage_mediator.clone(),
            core.id_mapping_service.clone(),
            core.path_service.clone(),
        ));

        // Actualizar el holder para el mediador una vez que el repository está creado
        if let Ok(mut holder) = folder_repository_holder.write() {
            *holder = Some(folder_repository.clone());
        }

        // Metadata cache
        let metadata_cache = Arc::new(FileMetadataCache::default_with_config(core.config.clone()));

        // Componentes refactorizados
        let metadata_manager = Arc::new(FileMetadataManager::new(
            metadata_cache.clone(),
            core.config.clone(),
        ));

        let path_resolver = Arc::new(FilePathResolver::new(
            core.path_service.clone(),
            storage_mediator.clone(),
            core.id_mapping_service.clone(),
        ));

        // File repositories separados para lectura y escritura
        let file_read_repository = Arc::new(FileFsReadRepository::new(
            self.storage_path.clone(),
            metadata_manager.clone(),
            path_resolver.clone(),
            core.config.clone(),
            None, // processor will be added later if needed
        ));

        let file_write_repository = Arc::new(FileFsWriteRepository::new(
            self.storage_path.clone(),
            metadata_manager.clone(),
            path_resolver.clone(),
            storage_mediator.clone(),
            core.config.clone(),
            None, // processor will be added later if needed
        ));

        // Legacy file repository - mantenido por compatibilidad
        let file_repository = Arc::new(FileFsRepository::new(
            self.storage_path.clone(),
            storage_mediator.clone(),
            core.id_mapping_service.clone(),
            core.path_service.clone(),
            metadata_cache,
        ));

        // I18n repository
        let i18n_repository = Arc::new(FileSystemI18nService::new(self.locales_path.clone()));

        // Trash repository
        let trash_repository = if core.config.features.enable_trash {
            Some(Arc::new(TrashFsRepository::new(
                self.storage_path.as_path(),
                core.id_mapping_service.clone(),
            ))
                as Arc<
                    dyn crate::domain::repositories::trash_repository::TrashRepository,
                >)
        } else {
            None
        };

        RepositoryServices {
            folder_repository,
            file_repository,
            file_read_repository,
            file_write_repository,
            i18n_repository,
            storage_mediator,
            metadata_manager,
            path_resolver,
            trash_repository,
        }
    }

    /// Inicializa los servicios de aplicación
    #[allow(dead_code)]
    pub fn create_application_services(&self, repos: &RepositoryServices) -> ApplicationServices {
        // Servicios principales
        let folder_service = Arc::new(FolderService::new(repos.folder_repository.clone()));

        // Antiguo servicio único
        let file_service = Arc::new(FileService::new(repos.file_repository.clone()));

        // Nuevos servicios refactorizados
        let file_upload_service =
            Arc::new(FileUploadService::new(repos.file_write_repository.clone()));

        let file_retrieval_service = Arc::new(FileRetrievalService::new(
            repos.file_read_repository.clone(),
        ));

        let file_management_service = Arc::new(FileManagementService::new(
            repos.file_write_repository.clone(),
        ));

        let file_use_case_factory = Arc::new(AppFileUseCaseFactory::new(
            repos.file_read_repository.clone(),
            repos.file_write_repository.clone(),
        ));

        let i18n_service = Arc::new(I18nApplicationService::new(repos.i18n_repository.clone()));

        // Servicio de papelera (deshabilitado temporalmente)
        let trash_service = None; // La función de papelera está deshabilitada por defecto

        // Servicio de búsqueda (deshabilitado por defecto)
        let search_service = None; // La función de búsqueda se activa según la configuración

        ApplicationServices {
            folder_service,
            file_service,
            file_upload_service,
            file_retrieval_service,
            file_management_service,
            file_use_case_factory,
            i18n_service,
            trash_service,
            search_service,
            share_service: None,     // No share service by default
            favorites_service: None, // No favorites service by default
            recent_service: None,    // No recent service by default
        }
    }
}

/// Contenedor para servicios base
#[allow(dead_code)]
#[derive(Clone)]
pub struct CoreServices {
    pub path_service: Arc<PathService>,
    pub cache_manager: Arc<StorageCacheManager>,
    pub id_mapping_service: Arc<dyn crate::application::ports::outbound::IdMappingPort>,
    pub config: AppConfig,
}

/// Contenedor para servicios de repositorio
#[allow(dead_code)]
#[derive(Clone)]
pub struct RepositoryServices {
    pub folder_repository: Arc<dyn FolderStoragePort>,
    pub file_repository: Arc<dyn FileStoragePort>,
    pub file_read_repository: Arc<dyn FileReadPort>,
    pub file_write_repository: Arc<dyn FileWritePort>,
    pub i18n_repository: Arc<dyn I18nService>,
    pub storage_mediator: Arc<dyn StorageMediator>,
    pub metadata_manager: Arc<FileMetadataManager>,
    pub path_resolver: Arc<FilePathResolver>,
    pub trash_repository:
        Option<Arc<dyn crate::domain::repositories::trash_repository::TrashRepository>>,
}

/// Contenedor para servicios de aplicación
#[allow(dead_code)]
#[derive(Clone)]
pub struct ApplicationServices {
    pub folder_service: Arc<dyn FolderUseCase>,
    pub file_service: Arc<dyn FileUseCase>,
    pub file_upload_service: Arc<dyn FileUploadUseCase>,
    pub file_retrieval_service: Arc<dyn FileRetrievalUseCase>,
    pub file_management_service: Arc<dyn FileManagementUseCase>,
    pub file_use_case_factory: Arc<dyn FileUseCaseFactory>,
    pub i18n_service: Arc<I18nApplicationService>,
    pub trash_service: Option<Arc<dyn TrashUseCase>>,
    pub search_service: Option<Arc<dyn SearchUseCase>>,
    pub share_service: Option<Arc<dyn crate::application::ports::share_ports::ShareUseCase>>,
    pub favorites_service: Option<Arc<dyn FavoritesUseCase>>,
    pub recent_service: Option<Arc<dyn RecentItemsUseCase>>,
}

/// Contenedor para servicios de autenticación
#[allow(dead_code)]
#[derive(Clone)]
pub struct AuthServices {
    pub auth_service: Arc<AuthService>,
    pub auth_application_service: Arc<AuthApplicationService>,
}

/// Estado global de la aplicación para dependency injection
#[derive(Clone)]
pub struct AppState {
    pub core: CoreServices,
    pub repositories: RepositoryServices,
    pub applications: ApplicationServices,
    pub db_pool: Option<Arc<PgPool>>,
    pub auth_service: Option<AuthServices>,
    pub trash_service: Option<Arc<dyn TrashUseCase>>,
    pub share_service: Option<Arc<dyn crate::application::ports::share_ports::ShareUseCase>>,
    pub favorites_service: Option<Arc<dyn FavoritesUseCase>>,
    pub recent_service: Option<Arc<dyn RecentItemsUseCase>>,
    pub storage_usage_service:
        Option<Arc<dyn crate::application::ports::storage_ports::StorageUsagePort>>,
    pub calendar_service: Option<Arc<dyn crate::application::ports::storage_ports::StorageUseCase>>,
    pub contact_service: Option<Arc<dyn crate::application::ports::storage_ports::StorageUseCase>>,
}

impl Default for AppState {
    fn default() -> Self {
        // This is just a minimal stub version for auth middleware
        // We'll need to create proper instance in main.rs

        let config = crate::common::config::AppConfig::default();
        let path_service = Arc::new(crate::domain::services::path_service::PathService::new(
            std::path::PathBuf::from("./storage"),
        ));

        // Create stub service implementations
        struct DummyIdMappingService;
        #[async_trait::async_trait]
        impl crate::application::ports::outbound::IdMappingPort for DummyIdMappingService {
            async fn get_or_create_id(
                &self,
                _path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<String, crate::common::errors::DomainError> {
                Ok("dummy-id".to_string())
            }

            async fn get_path_by_id(
                &self,
                _id: &str,
            ) -> Result<
                crate::domain::services::path_service::StoragePath,
                crate::common::errors::DomainError,
            > {
                Ok(crate::domain::services::path_service::StoragePath::from_string("/"))
            }

            async fn update_path(
                &self,
                _id: &str,
                _new_path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn remove_id(&self, _id: &str) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn save_changes(&self) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }
        }

        struct DummyStorageMediator;
        #[async_trait::async_trait]
        impl crate::application::services::storage_mediator::StorageMediator for DummyStorageMediator {
            async fn get_folder_path(
                &self,
                _folder_id: &str,
            ) -> Result<
                std::path::PathBuf,
                crate::application::services::storage_mediator::StorageMediatorError,
            > {
                Ok(std::path::PathBuf::from("/tmp"))
            }

            async fn get_folder_storage_path(
                &self,
                _folder_id: &str,
            ) -> Result<
                crate::domain::services::path_service::StoragePath,
                crate::application::services::storage_mediator::StorageMediatorError,
            > {
                Ok(crate::domain::services::path_service::StoragePath::root())
            }

            async fn get_folder(
                &self,
                _folder_id: &str,
            ) -> Result<
                crate::domain::entities::folder::Folder,
                crate::application::services::storage_mediator::StorageMediatorError,
            > {
                Err(
                    crate::application::services::storage_mediator::StorageMediatorError::NotFound(
                        "Stub not implemented".to_string(),
                    ),
                )
            }

            async fn file_exists_at_path(
                &self,
                _path: &std::path::Path,
            ) -> Result<bool, crate::application::services::storage_mediator::StorageMediatorError>
            {
                Ok(false)
            }

            async fn file_exists_at_storage_path(
                &self,
                _storage_path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<bool, crate::application::services::storage_mediator::StorageMediatorError>
            {
                Ok(false)
            }

            async fn folder_exists_at_path(
                &self,
                _path: &std::path::Path,
            ) -> Result<bool, crate::application::services::storage_mediator::StorageMediatorError>
            {
                Ok(false)
            }

            async fn folder_exists_at_storage_path(
                &self,
                _storage_path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<bool, crate::application::services::storage_mediator::StorageMediatorError>
            {
                Ok(false)
            }

            fn resolve_path(&self, _relative_path: &std::path::Path) -> std::path::PathBuf {
                std::path::PathBuf::from("/tmp")
            }

            fn resolve_storage_path(
                &self,
                _storage_path: &crate::domain::services::path_service::StoragePath,
            ) -> std::path::PathBuf {
                std::path::PathBuf::from("/tmp")
            }

            async fn ensure_directory(
                &self,
                _path: &std::path::Path,
            ) -> Result<(), crate::application::services::storage_mediator::StorageMediatorError>
            {
                Ok(())
            }

            async fn ensure_storage_directory(
                &self,
                _storage_path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<(), crate::application::services::storage_mediator::StorageMediatorError>
            {
                Ok(())
            }
        }

        struct DummyFileReadPort;
        #[async_trait::async_trait]
        impl crate::application::ports::storage_ports::FileReadPort for DummyFileReadPort {
            async fn get_file(
                &self,
                _id: &str,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn list_files(
                &self,
                _folder_id: Option<&str>,
            ) -> Result<Vec<crate::domain::entities::file::File>, crate::common::errors::DomainError>
            {
                Ok(Vec::new())
            }

            async fn get_file_content(
                &self,
                _id: &str,
            ) -> Result<Vec<u8>, crate::common::errors::DomainError> {
                Ok(Vec::new())
            }

            async fn get_file_stream(
                &self,
                _id: &str,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
                crate::common::errors::DomainError,
            > {
                let empty_stream = futures::stream::empty::<Result<bytes::Bytes, std::io::Error>>();
                Ok(Box::new(empty_stream))
            }
        }

        struct DummyFileWritePort;
        #[async_trait::async_trait]
        impl crate::application::ports::storage_ports::FileWritePort for DummyFileWritePort {
            async fn save_file(
                &self,
                _name: String,
                _folder_id: Option<String>,
                _content_type: String,
                _content: Vec<u8>,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn move_file(
                &self,
                _file_id: &str,
                _target_folder_id: Option<String>,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn delete_file(
                &self,
                _id: &str,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn get_folder_details(
                &self,
                _folder_id: &str,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn get_folder_path_str(
                &self,
                _folder_id: &str,
            ) -> Result<String, crate::common::errors::DomainError> {
                Ok("/Mi Carpeta - dummy".to_string())
            }
        }

        struct DummyFileStoragePort;
        #[async_trait::async_trait]
        impl crate::application::ports::outbound::FileStoragePort for DummyFileStoragePort {
            async fn save_file(
                &self,
                _name: String,
                _folder_id: Option<String>,
                _content_type: String,
                _content: Vec<u8>,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn get_file(
                &self,
                _id: &str,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn list_files(
                &self,
                _folder_id: Option<&str>,
            ) -> Result<Vec<crate::domain::entities::file::File>, crate::common::errors::DomainError>
            {
                Ok(Vec::new())
            }

            async fn delete_file(
                &self,
                _id: &str,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn get_file_content(
                &self,
                _id: &str,
            ) -> Result<Vec<u8>, crate::common::errors::DomainError> {
                Ok(Vec::new())
            }

            async fn get_file_stream(
                &self,
                _id: &str,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
                crate::common::errors::DomainError,
            > {
                let empty_stream = futures::stream::empty::<Result<bytes::Bytes, std::io::Error>>();
                Ok(Box::new(empty_stream))
            }

            async fn move_file(
                &self,
                _file_id: &str,
                _target_folder_id: Option<String>,
            ) -> Result<crate::domain::entities::file::File, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::file::File::default())
            }

            async fn get_file_path(
                &self,
                _id: &str,
            ) -> Result<
                crate::domain::services::path_service::StoragePath,
                crate::common::errors::DomainError,
            > {
                Ok(crate::domain::services::path_service::StoragePath::from_string("/"))
            }

            async fn get_parent_folder_id(
                &self,
                _path: &str,
            ) -> Result<String, crate::common::errors::DomainError> {
                Ok("root".to_string())
            }

            async fn update_file_content(
                &self,
                _file_id: &str,
                _content: Vec<u8>,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }
        }

        struct DummyFolderStoragePort;
        #[async_trait::async_trait]
        impl crate::application::ports::outbound::FolderStoragePort for DummyFolderStoragePort {
            async fn create_folder(
                &self,
                _name: String,
                _parent_id: Option<String>,
            ) -> Result<crate::domain::entities::folder::Folder, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::folder::Folder::default())
            }

            async fn get_folder(
                &self,
                _id: &str,
            ) -> Result<crate::domain::entities::folder::Folder, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::folder::Folder::default())
            }

            async fn get_folder_by_path(
                &self,
                _storage_path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<crate::domain::entities::folder::Folder, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::folder::Folder::default())
            }

            async fn list_folders(
                &self,
                _parent_id: Option<&str>,
            ) -> Result<
                Vec<crate::domain::entities::folder::Folder>,
                crate::common::errors::DomainError,
            > {
                Ok(Vec::new())
            }

            async fn list_folders_paginated(
                &self,
                _parent_id: Option<&str>,
                _offset: usize,
                _limit: usize,
                _include_total: bool,
            ) -> Result<
                (Vec<crate::domain::entities::folder::Folder>, Option<usize>),
                crate::common::errors::DomainError,
            > {
                Ok((Vec::new(), Some(0)))
            }

            async fn rename_folder(
                &self,
                _id: &str,
                _new_name: String,
            ) -> Result<crate::domain::entities::folder::Folder, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::folder::Folder::default())
            }

            async fn move_folder(
                &self,
                _id: &str,
                _new_parent_id: Option<&str>,
            ) -> Result<crate::domain::entities::folder::Folder, crate::common::errors::DomainError>
            {
                Ok(crate::domain::entities::folder::Folder::default())
            }

            async fn delete_folder(
                &self,
                _id: &str,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn folder_exists(
                &self,
                _storage_path: &crate::domain::services::path_service::StoragePath,
            ) -> Result<bool, crate::common::errors::DomainError> {
                Ok(false)
            }

            async fn get_folder_path(
                &self,
                _id: &str,
            ) -> Result<
                crate::domain::services::path_service::StoragePath,
                crate::common::errors::DomainError,
            > {
                Ok(crate::domain::services::path_service::StoragePath::from_string("/"))
            }
        }

        // File path resolution is handled by other components

        struct DummyI18nService;
        #[async_trait::async_trait]
        impl crate::domain::services::i18n_service::I18nService for DummyI18nService {
            async fn translate(
                &self,
                _key: &str,
                _locale: crate::domain::services::i18n_service::Locale,
            ) -> crate::domain::services::i18n_service::I18nResult<String> {
                Ok(String::new())
            }

            async fn load_translations(
                &self,
                _locale: crate::domain::services::i18n_service::Locale,
            ) -> crate::domain::services::i18n_service::I18nResult<()> {
                Ok(())
            }

            async fn available_locales(
                &self,
            ) -> Vec<crate::domain::services::i18n_service::Locale> {
                vec![crate::domain::services::i18n_service::Locale::default()]
            }

            async fn is_supported(
                &self,
                _locale: crate::domain::services::i18n_service::Locale,
            ) -> bool {
                true
            }
        }

        struct DummyFolderUseCase;
        #[async_trait::async_trait]
        impl crate::application::ports::inbound::FolderUseCase for DummyFolderUseCase {
            async fn create_folder(
                &self,
                _dto: crate::application::dtos::folder_dto::CreateFolderDto,
            ) -> Result<
                crate::application::dtos::folder_dto::FolderDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::folder_dto::FolderDto::default())
            }

            async fn get_folder(
                &self,
                _id: &str,
            ) -> Result<
                crate::application::dtos::folder_dto::FolderDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::folder_dto::FolderDto::default())
            }

            async fn get_folder_by_path(
                &self,
                _path: &str,
            ) -> Result<
                crate::application::dtos::folder_dto::FolderDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::folder_dto::FolderDto::default())
            }

            async fn list_folders(
                &self,
                _parent_id: Option<&str>,
            ) -> Result<
                Vec<crate::application::dtos::folder_dto::FolderDto>,
                crate::common::errors::DomainError,
            > {
                Ok(Vec::new())
            }

            async fn list_folders_paginated(
                &self,
                _parent_id: Option<&str>,
                _pagination: &crate::application::dtos::pagination::PaginationRequestDto,
            ) -> Result<
                crate::application::dtos::pagination::PaginatedResponseDto<
                    crate::application::dtos::folder_dto::FolderDto,
                >,
                crate::common::errors::DomainError,
            > {
                Ok(
                    crate::application::dtos::pagination::PaginatedResponseDto::new(
                        Vec::new(),
                        0,
                        10,
                        0,
                    ),
                )
            }

            async fn rename_folder(
                &self,
                _id: &str,
                _dto: crate::application::dtos::folder_dto::RenameFolderDto,
            ) -> Result<
                crate::application::dtos::folder_dto::FolderDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::folder_dto::FolderDto::default())
            }

            async fn move_folder(
                &self,
                _id: &str,
                _dto: crate::application::dtos::folder_dto::MoveFolderDto,
            ) -> Result<
                crate::application::dtos::folder_dto::FolderDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::folder_dto::FolderDto::default())
            }

            async fn delete_folder(
                &self,
                _id: &str,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }
        }

        struct DummyFileUseCase;
        #[async_trait::async_trait]
        impl crate::application::ports::inbound::FileUseCase for DummyFileUseCase {
            async fn upload_file(
                &self,
                _name: String,
                _folder_id: Option<String>,
                _content_type: String,
                _content: Vec<u8>,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }

            async fn get_file(
                &self,
                _id: &str,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }

            async fn get_file_by_path(
                &self,
                _path: &str,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }

            async fn create_file(
                &self,
                _parent_path: &str,
                _filename: &str,
                _content: &[u8],
                _content_type: &str,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }

            async fn update_file(
                &self,
                _path: &str,
                _content: &[u8],
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn list_files(
                &self,
                _folder_id: Option<&str>,
            ) -> Result<
                Vec<crate::application::dtos::file_dto::FileDto>,
                crate::common::errors::DomainError,
            > {
                Ok(Vec::new())
            }

            async fn delete_file(
                &self,
                _id: &str,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }

            async fn get_file_content(
                &self,
                _id: &str,
            ) -> Result<Vec<u8>, crate::common::errors::DomainError> {
                Ok(Vec::new())
            }

            async fn get_file_stream(
                &self,
                _id: &str,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
                crate::common::errors::DomainError,
            > {
                // Create an empty stream
                let empty_stream = futures::stream::empty::<Result<bytes::Bytes, std::io::Error>>();
                Ok(Box::new(empty_stream))
            }

            async fn move_file(
                &self,
                _file_id: &str,
                _folder_id: Option<String>,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }
        }

        struct DummyFileUploadUseCase;
        #[async_trait::async_trait]
        impl crate::application::ports::file_ports::FileUploadUseCase for DummyFileUploadUseCase {
            async fn upload_file(
                &self,
                _name: String,
                _folder_id: Option<String>,
                _content_type: String,
                _content: Vec<u8>,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }
        }

        struct DummyFileRetrievalUseCase;
        #[async_trait::async_trait]
        impl crate::application::ports::file_ports::FileRetrievalUseCase for DummyFileRetrievalUseCase {
            async fn get_file(
                &self,
                _id: &str,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }

            async fn list_files(
                &self,
                _folder_id: Option<&str>,
            ) -> Result<
                Vec<crate::application::dtos::file_dto::FileDto>,
                crate::common::errors::DomainError,
            > {
                Ok(Vec::new())
            }

            async fn get_file_content(
                &self,
                _id: &str,
            ) -> Result<Vec<u8>, crate::common::errors::DomainError> {
                Ok(Vec::new())
            }

            async fn get_file_stream(
                &self,
                _id: &str,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
                crate::common::errors::DomainError,
            > {
                // Create an empty stream
                let empty_stream = futures::stream::empty::<Result<bytes::Bytes, std::io::Error>>();
                Ok(Box::new(empty_stream))
            }
        }

        struct DummyFileManagementUseCase;
        #[async_trait::async_trait]
        impl crate::application::ports::file_ports::FileManagementUseCase for DummyFileManagementUseCase {
            async fn move_file(
                &self,
                _file_id: &str,
                _folder_id: Option<String>,
            ) -> Result<
                crate::application::dtos::file_dto::FileDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::file_dto::FileDto::default())
            }

            async fn delete_file(
                &self,
                _id: &str,
            ) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }
        }

        struct DummyFileUseCaseFactory;
        impl crate::application::ports::file_ports::FileUseCaseFactory for DummyFileUseCaseFactory {
            fn create_file_upload_use_case(
                &self,
            ) -> std::sync::Arc<dyn crate::application::ports::file_ports::FileUploadUseCase>
            {
                std::sync::Arc::new(DummyFileUploadUseCase)
            }

            fn create_file_retrieval_use_case(
                &self,
            ) -> std::sync::Arc<dyn crate::application::ports::file_ports::FileRetrievalUseCase>
            {
                std::sync::Arc::new(DummyFileRetrievalUseCase)
            }

            fn create_file_management_use_case(
                &self,
            ) -> std::sync::Arc<dyn crate::application::ports::file_ports::FileManagementUseCase>
            {
                std::sync::Arc::new(DummyFileManagementUseCase)
            }
        }

        struct DummyI18nApplicationService {}

        // Need to implement the actual service to match the type signature in DI container
        impl DummyI18nApplicationService {
            fn dummy(
            ) -> crate::application::services::i18n_application_service::I18nApplicationService
            {
                // We need to create an actual I18nApplicationService
                crate::application::services::i18n_application_service::I18nApplicationService::new(
                    Arc::new(DummyI18nService)
                        as Arc<dyn crate::domain::services::i18n_service::I18nService>,
                )
            }
        }

        // Create service instances
        let id_mapping_service = Arc::new(DummyIdMappingService)
            as Arc<dyn crate::application::ports::outbound::IdMappingPort>;
        let storage_mediator = Arc::new(DummyStorageMediator)
            as Arc<dyn crate::application::services::storage_mediator::StorageMediator>;
        let i18n_repository = Arc::new(DummyI18nService)
            as Arc<dyn crate::domain::services::i18n_service::I18nService>;
        let folder_service = Arc::new(DummyFolderUseCase)
            as Arc<dyn crate::application::ports::inbound::FolderUseCase>;
        let file_service =
            Arc::new(DummyFileUseCase) as Arc<dyn crate::application::ports::inbound::FileUseCase>;
        let file_upload_service = Arc::new(DummyFileUploadUseCase)
            as Arc<dyn crate::application::ports::file_ports::FileUploadUseCase>;
        let file_retrieval_service = Arc::new(DummyFileRetrievalUseCase)
            as Arc<dyn crate::application::ports::file_ports::FileRetrievalUseCase>;
        let file_management_service = Arc::new(DummyFileManagementUseCase)
            as Arc<dyn crate::application::ports::file_ports::FileManagementUseCase>;
        let file_use_case_factory = Arc::new(DummyFileUseCaseFactory)
            as Arc<dyn crate::application::ports::file_ports::FileUseCaseFactory>;

        // This creates the core services needed for basic functionality
        let core_services = CoreServices {
            path_service: path_service.clone(),
            cache_manager: Arc::new(
                crate::infrastructure::services::cache_manager::StorageCacheManager::default(),
            ),
            id_mapping_service: id_mapping_service.clone(),
            config: config.clone(),
        };

        // Create empty repository implementations
        let repository_services = RepositoryServices {
            folder_repository: Arc::new(DummyFolderStoragePort)
                as Arc<dyn crate::application::ports::outbound::FolderStoragePort>,
            file_repository: Arc::new(DummyFileStoragePort)
                as Arc<dyn crate::application::ports::outbound::FileStoragePort>,
            file_read_repository: Arc::new(DummyFileReadPort)
                as Arc<dyn crate::application::ports::storage_ports::FileReadPort>,
            file_write_repository: Arc::new(DummyFileWritePort)
                as Arc<dyn crate::application::ports::storage_ports::FileWritePort>,
            i18n_repository,
            storage_mediator: storage_mediator.clone(),
            metadata_manager: Arc::new(
                crate::infrastructure::repositories::FileMetadataManager::default(),
            ),
            path_resolver: Arc::new(
                crate::infrastructure::repositories::file_path_resolver::FilePathResolver::new(
                    path_service.clone(),
                    storage_mediator.clone(),
                    id_mapping_service.clone(),
                ),
            ),
            trash_repository: None, // No trash repository in minimal mode
        };

        // Create dummy search use case
        struct DummySearchUseCase;
        #[async_trait::async_trait]
        impl crate::application::ports::inbound::SearchUseCase for DummySearchUseCase {
            async fn search(
                &self,
                _criteria: crate::application::dtos::search_dto::SearchCriteriaDto,
            ) -> Result<
                crate::application::dtos::search_dto::SearchResultsDto,
                crate::common::errors::DomainError,
            > {
                Ok(crate::application::dtos::search_dto::SearchResultsDto::empty())
            }

            async fn clear_search_cache(&self) -> Result<(), crate::common::errors::DomainError> {
                Ok(())
            }
        }

        // Create application services
        let application_services = ApplicationServices {
            folder_service,
            file_service,
            file_upload_service,
            file_retrieval_service,
            file_management_service,
            file_use_case_factory,
            i18n_service: Arc::new(DummyI18nApplicationService::dummy()),
            trash_service: None, // No trash service in minimal mode
            search_service: Some(Arc::new(DummySearchUseCase)
                as Arc<dyn crate::application::ports::inbound::SearchUseCase>),
            share_service: None,     // No share service in minimal mode
            favorites_service: None, // No favorites service in minimal mode
            recent_service: None,    // No recent service in minimal mode
        };

        // Return a minimal app state
        Self {
            core: core_services,
            repositories: repository_services,
            applications: application_services,
            db_pool: None,
            auth_service: None,
            trash_service: None,
            share_service: None,
            favorites_service: None,
            recent_service: None,
            storage_usage_service: None,
            calendar_service: None,
            contact_service: None,
        }
    }
}

impl AppState {
    pub fn new(
        core: CoreServices,
        repositories: RepositoryServices,
        applications: ApplicationServices,
    ) -> Self {
        Self {
            core,
            repositories,
            applications,
            db_pool: None,
            auth_service: None,
            trash_service: None,
            share_service: None,
            favorites_service: None,
            recent_service: None,
            storage_usage_service: None,
            calendar_service: None,
            contact_service: None,
        }
    }

    pub fn with_database(mut self, db_pool: Arc<PgPool>) -> Self {
        self.db_pool = Some(db_pool);
        self
    }

    pub fn with_auth_services(mut self, auth_services: AuthServices) -> Self {
        self.auth_service = Some(auth_services);
        self
    }

    pub fn with_trash_service(mut self, trash_service: Arc<dyn TrashUseCase>) -> Self {
        self.trash_service = Some(trash_service);
        self
    }

    pub fn with_share_service(
        mut self,
        share_service: Arc<dyn crate::application::ports::share_ports::ShareUseCase>,
    ) -> Self {
        self.share_service = Some(share_service);
        self
    }

    pub fn with_favorites_service(mut self, favorites_service: Arc<dyn FavoritesUseCase>) -> Self {
        self.favorites_service = Some(favorites_service);
        self
    }

    pub fn with_recent_service(mut self, recent_service: Arc<dyn RecentItemsUseCase>) -> Self {
        self.recent_service = Some(recent_service);
        self
    }

    pub fn with_storage_usage_service(
        mut self,
        storage_usage_service: Arc<dyn crate::application::ports::storage_ports::StorageUsagePort>,
    ) -> Self {
        self.storage_usage_service = Some(storage_usage_service);
        self
    }

    pub fn with_calendar_service(
        mut self,
        calendar_service: Arc<dyn crate::application::ports::storage_ports::StorageUseCase>,
    ) -> Self {
        self.calendar_service = Some(calendar_service);
        self
    }

    pub fn with_contact_service(
        mut self,
        contact_service: Arc<dyn crate::application::ports::storage_ports::StorageUseCase>,
    ) -> Self {
        self.contact_service = Some(contact_service);
        self
    }
}
