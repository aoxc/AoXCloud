use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::sync::Arc;

use crate::application::dtos::file_dto::FileDto;
use crate::application::dtos::folder_dto::{
    CreateFolderDto, FolderDto, MoveFolderDto, RenameFolderDto,
};
use crate::application::dtos::search_dto::{SearchCriteriaDto, SearchResultsDto};
use crate::common::errors::DomainError;

/// Puerto primario para operaciones de archivos
#[async_trait]
pub trait FileUseCase: Send + Sync + 'static {
    /// Sube un nuevo archivo desde bytes
    async fn upload_file(
        &self,
        name: String,
        folder_id: Option<String>,
        content_type: String,
        content: Vec<u8>,
    ) -> Result<FileDto, DomainError>;

    /// Obtiene un archivo por su ID
    async fn get_file(&self, id: &str) -> Result<FileDto, DomainError>;

    /// Obtiene un archivo por su ruta (para WebDAV)
    async fn get_file_by_path(&self, path: &str) -> Result<FileDto, DomainError>;

    /// Crea un nuevo archivo en la ruta especificada (para WebDAV)
    async fn create_file(
        &self,
        parent_path: &str,
        filename: &str,
        content: &[u8],
        content_type: &str,
    ) -> Result<FileDto, DomainError>;

    /// Actualiza el contenido de un archivo existente (para WebDAV)
    async fn update_file(&self, path: &str, content: &[u8]) -> Result<(), DomainError>;

    /// Lista archivos en una carpeta
    async fn list_files(&self, folder_id: Option<&str>) -> Result<Vec<FileDto>, DomainError>;

    /// Elimina un archivo
    async fn delete_file(&self, id: &str) -> Result<(), DomainError>;

    /// Obtiene contenido de archivo como bytes (para archivos pequeños)
    async fn get_file_content(&self, id: &str) -> Result<Vec<u8>, DomainError>;

    /// Obtiene contenido de archivo como stream (para archivos grandes)
    async fn get_file_stream(
        &self,
        id: &str,
    ) -> Result<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>, DomainError>;

    /// Mueve un archivo a otra carpeta
    async fn move_file(
        &self,
        file_id: &str,
        folder_id: Option<String>,
    ) -> Result<FileDto, DomainError>;
}

/// Puerto primario para operaciones de carpetas
#[async_trait]
pub trait FolderUseCase: Send + Sync + 'static {
    /// Crea una nueva carpeta
    async fn create_folder(&self, dto: CreateFolderDto) -> Result<FolderDto, DomainError>;

    /// Obtiene una carpeta por su ID
    async fn get_folder(&self, id: &str) -> Result<FolderDto, DomainError>;

    /// Obtiene una carpeta por su ruta
    async fn get_folder_by_path(&self, path: &str) -> Result<FolderDto, DomainError>;

    /// Lista carpetas dentro de una carpeta padre
    async fn list_folders(&self, parent_id: Option<&str>) -> Result<Vec<FolderDto>, DomainError>;

    /// Lista carpetas con paginación
    async fn list_folders_paginated(
        &self,
        parent_id: Option<&str>,
        pagination: &crate::application::dtos::pagination::PaginationRequestDto,
    ) -> Result<crate::application::dtos::pagination::PaginatedResponseDto<FolderDto>, DomainError>;

    /// Renombra una carpeta
    async fn rename_folder(&self, id: &str, dto: RenameFolderDto)
        -> Result<FolderDto, DomainError>;

    /// Mueve una carpeta a otro padre
    async fn move_folder(&self, id: &str, dto: MoveFolderDto) -> Result<FolderDto, DomainError>;

    /// Elimina una carpeta
    async fn delete_folder(&self, id: &str) -> Result<(), DomainError>;
}

/**
 * Puerto primario para búsqueda de archivos y carpetas
 *
 * Define las operaciones relacionadas con la búsqueda avanzada de
 * archivos y carpetas basándose en diversos criterios.
 */
#[async_trait]
pub trait SearchUseCase: Send + Sync + 'static {
    /**
     * Realiza una búsqueda basada en los criterios especificados
     *
     * @param criteria Criterios de búsqueda que incluyen texto, fechas, tamaños, etc.
     * @return Resultados de la búsqueda que contienen archivos y carpetas coincidentes
     */
    async fn search(&self, criteria: SearchCriteriaDto) -> Result<SearchResultsDto, DomainError>;

    /**
     * Limpia la caché de resultados de búsqueda
     *
     * @return Resultado indicando éxito o error
     */
    async fn clear_search_cache(&self) -> Result<(), DomainError>;
}

/// Factory para crear implementaciones de casos de uso
pub trait UseCaseFactory {
    fn create_file_use_case(&self) -> Arc<dyn FileUseCase>;
    fn create_folder_use_case(&self) -> Arc<dyn FolderUseCase>;
    fn create_search_use_case(&self) -> Arc<dyn SearchUseCase>;
}
