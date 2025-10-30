use std::sync::Arc;

use crate::application::ports::file_ports::{
    FileManagementUseCase, FileRetrievalUseCase, FileUploadUseCase, FileUseCaseFactory,
};
use crate::application::ports::storage_ports::{FileReadPort, FileWritePort};
use crate::application::services::file_management_service::FileManagementService;
use crate::application::services::file_retrieval_service::FileRetrievalService;
use crate::application::services::file_upload_service::FileUploadService;

/// Factory para crear implementaciones de casos de uso de archivos
pub struct AppFileUseCaseFactory {
    file_read_repository: Arc<dyn FileReadPort>,
    file_write_repository: Arc<dyn FileWritePort>,
}

impl AppFileUseCaseFactory {
    /// Crea una nueva factory para casos de uso de archivos
    pub fn new(
        file_read_repository: Arc<dyn FileReadPort>,
        file_write_repository: Arc<dyn FileWritePort>,
    ) -> Self {
        Self {
            file_read_repository,
            file_write_repository,
        }
    }

    /// Crea un stub para pruebas
    pub fn default_stub() -> Self {
        Self {
            file_read_repository: Arc::new(
                crate::infrastructure::repositories::FileFsReadRepository::default_stub(),
            ),
            file_write_repository: Arc::new(
                crate::infrastructure::repositories::FileFsWriteRepository::default_stub(),
            ),
        }
    }
}

impl FileUseCaseFactory for AppFileUseCaseFactory {
    fn create_file_upload_use_case(&self) -> Arc<dyn FileUploadUseCase> {
        Arc::new(FileUploadService::new(self.file_write_repository.clone()))
    }

    fn create_file_retrieval_use_case(&self) -> Arc<dyn FileRetrievalUseCase> {
        Arc::new(FileRetrievalService::new(self.file_read_repository.clone()))
    }

    fn create_file_management_use_case(&self) -> Arc<dyn FileManagementUseCase> {
        Arc::new(FileManagementService::new(
            self.file_write_repository.clone(),
        ))
    }
}
