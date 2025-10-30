use crate::application::dtos::favorites_dto::FavoriteItemDto;
use crate::application::ports::favorites_ports::FavoritesUseCase;
use crate::common::errors::{DomainError, ErrorKind, Result};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Implementation of the FavoritesUseCase for managing user favorites
pub struct FavoritesService {
    db_pool: Arc<PgPool>,
}

impl FavoritesService {
    /// Create a new FavoritesService with the given database pool
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl FavoritesUseCase for FavoritesService {
    /// Get all favorites for a user
    async fn get_favorites(&self, user_id: &str) -> Result<Vec<FavoriteItemDto>> {
        info!("Getting favorites for user: {}", user_id);

        // Parse user ID as UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Execute raw query to avoid sqlx macros issues
        let rows = sqlx::query(
            r#"
            SELECT 
                id::TEXT as "id", 
                user_id::TEXT as "user_id", 
                item_id as "item_id", 
                item_type as "item_type", 
                created_at as "created_at"
            FROM auth.user_favorites 
            WHERE user_id = $1::TEXT
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_uuid)
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|e| {
            error!("Database error fetching favorites: {}", e);
            DomainError::new(
                ErrorKind::InternalError,
                "Favorites",
                format!("Failed to fetch favorites: {}", e),
            )
        })?;

        // Map rows to DTOs
        let mut favorites = Vec::with_capacity(rows.len());
        for row in rows {
            favorites.push(FavoriteItemDto {
                id: row.get("id"),
                user_id: row.get("user_id"),
                item_id: row.get("item_id"),
                item_type: row.get("item_type"),
                created_at: row.get("created_at"),
            });
        }

        info!(
            "Retrieved {} favorites for user {}",
            favorites.len(),
            user_id
        );
        Ok(favorites)
    }

    /// Add an item to user's favorites
    async fn add_to_favorites(&self, user_id: &str, item_id: &str, item_type: &str) -> Result<()> {
        info!(
            "Adding {} '{}' to favorites for user {}",
            item_type, item_id, user_id
        );

        // Validate item_type
        if item_type != "file" && item_type != "folder" {
            return Err(DomainError::new(
                ErrorKind::InvalidInput,
                "Favorites",
                "Item type must be 'file' or 'folder'",
            ));
        }

        // Parse user ID as UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Execute raw query to avoid sqlx macros issues
        sqlx::query(
            r#"
            INSERT INTO auth.user_favorites (user_id, item_id, item_type)
            VALUES ($1::TEXT, $2, $3)
            ON CONFLICT (user_id, item_id, item_type) DO NOTHING
            "#,
        )
        .bind(user_uuid)
        .bind(item_id)
        .bind(item_type)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| {
            error!("Database error adding favorite: {}", e);
            DomainError::new(
                ErrorKind::InternalError,
                "Favorites",
                format!("Failed to add to favorites: {}", e),
            )
        })?;

        info!(
            "Successfully added {} '{}' to favorites for user {}",
            item_type, item_id, user_id
        );
        Ok(())
    }

    /// Remove an item from user's favorites
    async fn remove_from_favorites(
        &self,
        user_id: &str,
        item_id: &str,
        item_type: &str,
    ) -> Result<bool> {
        info!(
            "Removing {} '{}' from favorites for user {}",
            item_type, item_id, user_id
        );

        // Parse user ID as UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Execute raw query to avoid sqlx macros issues
        let result = sqlx::query(
            r#"
            DELETE FROM auth.user_favorites
            WHERE user_id = $1::TEXT AND item_id = $2 AND item_type = $3
            "#,
        )
        .bind(user_uuid)
        .bind(item_id)
        .bind(item_type)
        .execute(&*self.db_pool)
        .await
        .map_err(|e| {
            error!("Database error removing favorite: {}", e);
            DomainError::new(
                ErrorKind::InternalError,
                "Favorites",
                format!("Failed to remove from favorites: {}", e),
            )
        })?;

        let removed = result.rows_affected() > 0;
        info!(
            "{} {} '{}' from favorites for user {}",
            if removed {
                "Successfully removed"
            } else {
                "Did not find"
            },
            item_type,
            item_id,
            user_id
        );

        Ok(removed)
    }

    /// Check if an item is in user's favorites
    async fn is_favorite(&self, user_id: &str, item_id: &str, item_type: &str) -> Result<bool> {
        info!(
            "Checking if {} '{}' is favorite for user {}",
            item_type, item_id, user_id
        );

        // Parse user ID as UUID
        let user_uuid = Uuid::parse_str(user_id)?;

        // Execute raw query to avoid sqlx macros issues
        let row = sqlx::query(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM auth.user_favorites
                WHERE user_id = $1::TEXT AND item_id = $2 AND item_type = $3
            ) AS "is_favorite"
            "#,
        )
        .bind(user_uuid)
        .bind(item_id)
        .bind(item_type)
        .fetch_one(&*self.db_pool)
        .await
        .map_err(|e| {
            error!("Database error checking favorite status: {}", e);
            DomainError::new(
                ErrorKind::InternalError,
                "Favorites",
                format!("Failed to check favorite status: {}", e),
            )
        })?;

        // Get the boolean value from the row
        let is_favorite: bool = row.try_get("is_favorite").unwrap_or(false);

        Ok(is_favorite)
    }
}
