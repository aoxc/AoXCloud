use crate::application::dtos::recent_dto::RecentItemDto;
use crate::common::errors::Result;
use async_trait::async_trait;

/// Define operaciones para gestionar elementos recientes del usuario
#[async_trait]
pub trait RecentItemsUseCase: Send + Sync {
    /// Obtener todos los elementos recientes de un usuario
    async fn get_recent_items(
        &self,
        user_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<RecentItemDto>>;

    /// Registrar acceso a un elemento
    async fn record_item_access(&self, user_id: &str, item_id: &str, item_type: &str)
        -> Result<()>;

    /// Eliminar un elemento de recientes
    async fn remove_from_recent(
        &self,
        user_id: &str,
        item_id: &str,
        item_type: &str,
    ) -> Result<bool>;

    /// Limpiar toda la lista de elementos recientes
    async fn clear_recent_items(&self, user_id: &str) -> Result<()>;
}
