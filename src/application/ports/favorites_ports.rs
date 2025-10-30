use crate::application::dtos::favorites_dto::FavoriteItemDto;
use crate::common::errors::Result;
use async_trait::async_trait;

/// Defines operations for managing user favorites
#[async_trait]
pub trait FavoritesUseCase: Send + Sync {
    /// Get all favorites for a user
    async fn get_favorites(&self, user_id: &str) -> Result<Vec<FavoriteItemDto>>;

    /// Add an item to user's favorites
    async fn add_to_favorites(&self, user_id: &str, item_id: &str, item_type: &str) -> Result<()>;

    /// Remove an item from user's favorites
    async fn remove_from_favorites(
        &self,
        user_id: &str,
        item_id: &str,
        item_type: &str,
    ) -> Result<bool>;

    /// Check if an item is in user's favorites
    async fn is_favorite(&self, user_id: &str, item_id: &str, item_type: &str) -> Result<bool>;
}
