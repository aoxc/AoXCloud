use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DTO para elementos recientes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentItemDto {
    /// Identificador único para el elemento reciente
    pub id: String,

    /// ID del usuario propietario
    pub user_id: String,

    /// ID del elemento (archivo o carpeta)
    pub item_id: String,

    /// Tipo del elemento ('file' o 'folder')
    pub item_type: String,

    /// Cuándo se accedió al elemento
    pub accessed_at: DateTime<Utc>,
}
