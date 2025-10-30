use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tracing::{error, info};

use crate::application::dtos::search_dto::SearchCriteriaDto;
use crate::common::di::AppState;

/**
 * Manejador para las operaciones de búsqueda a través de la API.
 *
 * Este manejador expone endpoints relacionados con la funcionalidad de búsqueda,
 * permitiendo a los usuarios buscar archivos y carpetas usando diversos criterios.
 */
pub struct SearchHandler;

impl SearchHandler {
    /**
     * Realiza una búsqueda basada en los criterios proporcionados como parámetros de consulta.
     *
     * Este endpoint permite búsquedas simples directamente con parámetros URL.
     *
     * @param state Estado de la aplicación con servicios
     * @param query_params Parámetros de búsqueda como query string
     * @return Respuesta HTTP con los resultados de la búsqueda
     */
    pub async fn search_files_get(
        State(state): State<AppState>,
        Query(params): Query<SearchParams>,
    ) -> impl IntoResponse {
        info!("API: Búsqueda de archivos con parámetros: {:?}", params);

        // Extraer el servicio de búsqueda o devolver error si no está disponible
        let search_service = match &state.applications.search_service {
            Some(service) => service,
            None => {
                error!("Servicio de búsqueda no disponible");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "error": "Search service is not available"
                    })),
                )
                    .into_response();
            }
        };

        // Convertir parámetros de búsqueda a DTO
        let search_criteria = SearchCriteriaDto {
            name_contains: params.query,
            file_types: params
                .type_filter
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect()),
            created_after: params.created_after,
            created_before: params.created_before,
            modified_after: params.modified_after,
            modified_before: params.modified_before,
            min_size: params.min_size,
            max_size: params.max_size,
            folder_id: params.folder_id,
            recursive: params.recursive.unwrap_or(true),
            limit: params.limit.unwrap_or(100),
            offset: params.offset.unwrap_or(0),
        };

        // Realizar la búsqueda
        match search_service.search(search_criteria).await {
            Ok(results) => {
                info!(
                    "Búsqueda completada, {} archivos y {} carpetas encontrados",
                    results.files.len(),
                    results.folders.len()
                );
                (StatusCode::OK, Json(results)).into_response()
            }
            Err(err) => {
                error!("Error en búsqueda: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Search error: {}", err)
                    })),
                )
                    .into_response()
            }
        }
    }

    /**
     * Realiza una búsqueda avanzada basada en un objeto de criterios JSON completo.
     *
     * Este endpoint permite búsquedas más complejas con todos los criterios posibles
     * proporcionados en el cuerpo de la solicitud.
     *
     * @param state Estado de la aplicación con servicios
     * @param criteria Criterios de búsqueda completos
     * @return Respuesta HTTP con los resultados de la búsqueda
     */
    pub async fn search_files_post(
        State(state): State<AppState>,
        Json(criteria): Json<SearchCriteriaDto>,
    ) -> impl IntoResponse {
        info!("API: Búsqueda avanzada de archivos");

        // Extraer el servicio de búsqueda o devolver error si no está disponible
        let search_service = match &state.applications.search_service {
            Some(service) => service,
            None => {
                error!("Servicio de búsqueda no disponible");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "error": "Search service is not available"
                    })),
                )
                    .into_response();
            }
        };

        // Realizar la búsqueda
        match search_service.search(criteria).await {
            Ok(results) => {
                info!(
                    "Búsqueda completada, {} archivos y {} carpetas encontrados",
                    results.files.len(),
                    results.folders.len()
                );
                (StatusCode::OK, Json(results)).into_response()
            }
            Err(err) => {
                error!("Error en búsqueda: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Search error: {}", err)
                    })),
                )
                    .into_response()
            }
        }
    }

    /**
     * Limpia la caché de resultados de búsqueda.
     *
     * Este endpoint es útil para forzar búsquedas frescas después de cambios
     * significativos en el sistema de archivos.
     *
     * @param state Estado de la aplicación con servicios
     * @return Respuesta HTTP indicando éxito o error
     */
    pub async fn clear_search_cache(State(state): State<AppState>) -> impl IntoResponse {
        info!("API: Limpiando caché de búsqueda");

        // Extraer el servicio de búsqueda o devolver error si no está disponible
        let search_service = match &state.applications.search_service {
            Some(service) => service,
            None => {
                error!("Servicio de búsqueda no disponible");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "error": "Search service is not available"
                    })),
                )
                    .into_response();
            }
        };

        // Limpiar la caché
        match search_service.clear_search_cache().await {
            Ok(_) => {
                info!("Caché de búsqueda limpiada correctamente");
                (
                    StatusCode::OK,
                    Json(json!({
                        "message": "Search cache cleared successfully"
                    })),
                )
                    .into_response()
            }
            Err(err) => {
                error!("Error al limpiar caché de búsqueda: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Error clearing search cache: {}", err)
                    })),
                )
                    .into_response()
            }
        }
    }
}

/// Parámetros de búsqueda para el endpoint GET
#[derive(Debug, serde::Deserialize)]
pub struct SearchParams {
    /// Texto a buscar en nombres de archivos y carpetas
    pub query: Option<String>,

    /// Filtro por tipos de archivo (extensiones separadas por comas)
    #[serde(rename = "type")]
    pub type_filter: Option<String>,

    /// Filtrar elementos creados después de esta fecha (timestamp)
    pub created_after: Option<u64>,

    /// Filtrar elementos creados antes de esta fecha (timestamp)
    pub created_before: Option<u64>,

    /// Filtrar elementos modificados después de esta fecha (timestamp)
    pub modified_after: Option<u64>,

    /// Filtrar elementos modificados antes de esta fecha (timestamp)
    pub modified_before: Option<u64>,

    /// Tamaño mínimo en bytes
    pub min_size: Option<u64>,

    /// Tamaño máximo en bytes
    pub max_size: Option<u64>,

    /// ID de carpeta para limitar la búsqueda
    pub folder_id: Option<String>,

    /// Búsqueda recursiva en subcarpetas
    pub recursive: Option<bool>,

    /// Límite de resultados para paginación
    pub limit: Option<usize>,

    /// Desplazamiento para paginación
    pub offset: Option<usize>,
}
