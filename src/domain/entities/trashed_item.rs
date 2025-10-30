use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum TrashedItemType {
    File,
    Folder,
}

#[derive(Debug, Clone)]
pub struct TrashedItem {
    pub id: Uuid,
    pub original_id: Uuid,
    pub user_id: Uuid,
    pub item_type: TrashedItemType,
    pub name: String,
    pub original_path: String,
    pub trashed_at: DateTime<Utc>,
    pub deletion_date: DateTime<Utc>, // Fecha de eliminación permanente automática
}

impl TrashedItem {
    pub fn new(
        original_id: Uuid,
        user_id: Uuid,
        item_type: TrashedItemType,
        name: String,
        original_path: String,
        retention_days: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            original_id,
            user_id,
            item_type,
            name,
            original_path,
            trashed_at: now,
            deletion_date: now + chrono::Duration::days(retention_days as i64),
        }
    }

    pub fn days_until_deletion(&self) -> i64 {
        let now = Utc::now();
        (self.deletion_date - now).num_days().max(0)
    }
}
