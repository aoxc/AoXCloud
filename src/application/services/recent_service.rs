use crate::application::dtos::recent_dto::RecentItemDto;
use crate::application::ports::recent_ports::RecentItemsUseCase;
use crate::common::errors::{DomainError, ErrorKind, Result};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Implementación del caso de uso para gestionar elementos recientes
pub struct RecentService {
    db_pool: Arc<PgPool>,
    max_recent_items: i32, // Número máximo de elementos recientes a mantener por usuario
}

impl RecentService {
    /// Crear un nuevo servicio de elementos recientes
    pub fn new(db_pool: Arc<PgPool>, max_recent_items: i32) -> Self {
        Self {
            db_pool,
            max_recent_items: max_recent_items.max(1).min(100), // Entre 1 y 100
        }
    }
}

#[async_trait]
impl RecentItemsUseCase for RecentService {
    /// Obtener elementos recientes de un usuario
    async fn get_recent_items(
        &self,
        user_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<RecentItemDto>> {
        info!("Obteniendo elementos recientes para usuario: {}", user_id);

        // Convertir user_id a UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Determinar límite (usar el especificado o el máximo del servicio)
        let limit_value = limit
            .unwrap_or(self.max_recent_items)
            .min(self.max_recent_items);

        // Ejecutar consulta SQL
        let rows = sqlx::query(
            r#"
            SELECT 
                id::TEXT as "id", 
                user_id::TEXT as "user_id", 
                item_id as "item_id", 
                item_type as "item_type", 
                accessed_at as "accessed_at"
            FROM auth.user_recent_files 
            WHERE user_id = $1::TEXT
            ORDER BY accessed_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_uuid)
        .bind(limit_value)
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|e| {
            error!(
                "Error de base de datos al obtener elementos recientes: {}",
                e
            );
            DomainError::new(
                ErrorKind::InternalError,
                "RecentItems",
                format!("Fallo al obtener elementos recientes: {}", e),
            )
        })?;

        // Convertir filas a DTOs
        let mut recent_items = Vec::with_capacity(rows.len());
        for row in rows {
            recent_items.push(RecentItemDto {
                id: row.get("id"),
                user_id: row.get("user_id"),
                item_id: row.get("item_id"),
                item_type: row.get("item_type"),
                accessed_at: row.get("accessed_at"),
            });
        }

        info!(
            "Recuperados {} elementos recientes para usuario {}",
            recent_items.len(),
            user_id
        );
        Ok(recent_items)
    }

    /// Registrar acceso a un elemento
    async fn record_item_access(
        &self,
        user_id: &str,
        item_id: &str,
        item_type: &str,
    ) -> Result<()> {
        info!(
            "Registrando acceso a {} '{}' para usuario {}",
            item_type, item_id, user_id
        );

        // Validar tipo de elemento
        if item_type != "file" && item_type != "folder" {
            return Err(DomainError::new(
                ErrorKind::InvalidInput,
                "RecentItems",
                "El tipo de elemento debe ser 'file' o 'folder'",
            ));
        }

        // Convertir user_id a UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Ejecutar consulta SQL con UPSERT para mantener un único registro por elemento
        sqlx::query(
            r#"
            INSERT INTO auth.user_recent_files (user_id, item_id, item_type, accessed_at)
            VALUES ($1::TEXT, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (user_id, item_id, item_type) 
            DO UPDATE SET accessed_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(user_uuid)
        .bind(item_id)
        .bind(item_type)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| {
            error!(
                "Error de base de datos al registrar acceso a elemento: {}",
                e
            );
            DomainError::new(
                ErrorKind::InternalError,
                "RecentItems",
                format!("Fallo al registrar acceso a elemento: {}", e),
            )
        })?;

        // Eliminar elementos antiguos que excedan el límite
        self.prune_old_items(user_id).await?;

        info!(
            "Registrado correctamente acceso a {} '{}' para usuario {}",
            item_type, item_id, user_id
        );
        Ok(())
    }

    /// Eliminar un elemento de recientes
    async fn remove_from_recent(
        &self,
        user_id: &str,
        item_id: &str,
        item_type: &str,
    ) -> Result<bool> {
        info!(
            "Eliminando {} '{}' de recientes para usuario {}",
            item_type, item_id, user_id
        );

        // Convertir user_id a UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Ejecutar consulta SQL
        let result = sqlx::query(
            r#"
            DELETE FROM auth.user_recent_files
            WHERE user_id = $1::TEXT AND item_id = $2 AND item_type = $3
            "#,
        )
        .bind(user_uuid)
        .bind(item_id)
        .bind(item_type)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| {
            error!(
                "Error de base de datos al eliminar elemento de recientes: {}",
                e
            );
            DomainError::new(
                ErrorKind::InternalError,
                "RecentItems",
                format!("Fallo al eliminar de recientes: {}", e),
            )
        })?;

        let removed = result.rows_affected() > 0;
        info!(
            "{} {} '{}' de recientes para usuario {}",
            if removed {
                "Eliminado correctamente"
            } else {
                "No se encontró"
            },
            item_type,
            item_id,
            user_id
        );

        Ok(removed)
    }

    /// Limpiar todos los elementos recientes
    async fn clear_recent_items(&self, user_id: &str) -> Result<()> {
        info!(
            "Limpiando todos los elementos recientes para usuario {}",
            user_id
        );

        // Convertir user_id a UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Ejecutar consulta SQL
        sqlx::query(
            r#"
            DELETE FROM auth.user_recent_files
            WHERE user_id = $1::TEXT
            "#,
        )
        .bind(user_uuid)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| {
            error!(
                "Error de base de datos al limpiar elementos recientes: {}",
                e
            );
            DomainError::new(
                ErrorKind::InternalError,
                "RecentItems",
                format!("Fallo al limpiar elementos recientes: {}", e),
            )
        })?;

        info!(
            "Limpiados todos los elementos recientes para usuario {}",
            user_id
        );
        Ok(())
    }
}

impl RecentService {
    /// Método auxiliar para eliminar elementos antiguos que excedan el límite
    async fn prune_old_items(&self, user_id: &str) -> Result<()> {
        // Convertir user_id a UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Eliminar elementos antiguos que excedan el límite
        sqlx::query(
            r#"
            DELETE FROM auth.user_recent_files
            WHERE id IN (
                SELECT id FROM auth.user_recent_files
                WHERE user_id = $1::TEXT
                ORDER BY accessed_at DESC
                OFFSET $2
            )
            "#,
        )
        .bind(user_uuid)
        .bind(self.max_recent_items)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| {
            error!("Error al podar elementos recientes antiguos: {}", e);
            DomainError::new(
                ErrorKind::InternalError,
                "RecentItems",
                format!("Fallo al limpiar elementos recientes antiguos: {}", e),
            )
        })?;

        Ok(())
    }
}
