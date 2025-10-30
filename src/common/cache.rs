use crate::common::errors::{DomainError, ErrorKind};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Entrada de caché con tiempo de expiración
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CacheEntry<V> {
    value: V,
    expiry: Instant,
}

impl<V> CacheEntry<V> {
    /// Crea una nueva entrada en la caché
    #[allow(dead_code)]
    fn new(value: V, ttl: Duration) -> Self {
        Self {
            value,
            expiry: Instant::now() + ttl,
        }
    }

    /// Verifica si la entrada ha expirado
    #[allow(dead_code)]
    fn is_expired(&self) -> bool {
        Instant::now() > self.expiry
    }
}

/// Servicio genérico de caché con TTL
#[allow(dead_code)]
pub struct CacheService<K, V> {
    cache: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    ttl: Duration,
    max_entries: usize,
}

impl<K, V> CacheService<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static + std::fmt::Debug,
    V: Clone + Send + Sync + 'static,
{
    /// Crea un nuevo servicio de caché
    #[allow(dead_code)]
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            max_entries,
        }
    }

    /// Obtiene un valor de la caché o lo inserta si no existe
    #[allow(dead_code)]
    pub async fn get_or_insert<F, E>(&self, key: K, loader: F) -> Result<V, DomainError>
    where
        F: FnOnce() -> Result<V, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Intentar leer de la caché primero
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&key) {
                if !entry.is_expired() {
                    tracing::debug!("Cache hit for key: {:?}", key);
                    return Ok(entry.value.clone());
                }
            }
        }

        // Cache miss o entrada expirada, obtener valor y actualizar
        let value = loader().map_err(|e| {
            DomainError::new(
                ErrorKind::InternalError,
                "Cache",
                format!("Failed to load value for cache: {}", e),
            )
            .with_source(e)
        })?;

        // Insertar en la caché
        {
            let mut cache = self.cache.write().await;

            // Si alcanzamos el límite, eliminar una entrada aleatoria
            if cache.len() >= self.max_entries {
                if let Some(expired_key) = cache
                    .iter()
                    .find(|(_, v)| v.is_expired())
                    .map(|(k, _)| k.clone())
                {
                    cache.remove(&expired_key);
                } else if let Some(random_key) = cache.keys().next().cloned() {
                    cache.remove(&random_key);
                }
            }

            cache.insert(key.clone(), CacheEntry::new(value.clone(), self.ttl));
        }

        tracing::debug!("Cache miss for key: {:?}, value loaded and cached", key);
        Ok(value)
    }

    /// Invalida una entrada específica de la caché
    #[allow(dead_code)]
    pub async fn invalidate(&self, key: &K) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
        tracing::debug!("Cache entry invalidated for key: {:?}", key);
    }

    /// Invalida todas las entradas de la caché
    #[allow(dead_code)]
    pub async fn invalidate_all(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::debug!("Cache fully invalidated");
    }

    /// Obtiene el número de entradas en la caché
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Limpia las entradas expiradas de la caché
    #[allow(dead_code)]
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_len = cache.len();

        cache.retain(|_, v| !v.is_expired());

        let removed = initial_len - cache.len();
        if removed > 0 {
            tracing::debug!("Removed {} expired cache entries", removed);
        }

        removed
    }
}

/// Caché específica para metadatos de archivos
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileMetadata {
    pub size: u64,
    pub created_at: u64,
    pub modified_at: u64,
    pub is_dir: bool,
}

/// Gestor de caché para operaciones comunes de almacenamiento
#[allow(dead_code)]
pub struct CacheManager {
    /// Caché para metadatos de archivos/carpetas
    metadata_cache: CacheService<std::path::PathBuf, FileMetadata>,
    /// Caché para verificación de existencia de archivos
    existence_cache: CacheService<std::path::PathBuf, bool>,
}

impl CacheManager {
    /// Crea un nuevo gestor de caché
    #[allow(dead_code)]
    pub fn new(metadata_ttl: Duration, existence_ttl: Duration) -> Self {
        Self {
            metadata_cache: CacheService::new(metadata_ttl, 10000), // Caché para 10,000 elementos
            existence_cache: CacheService::new(existence_ttl, 20000), // Caché para 20,000 elementos
        }
    }

    /// Obtiene o carga los metadatos de un archivo/carpeta
    #[allow(dead_code)]
    pub async fn get_metadata<F>(
        &self,
        path: std::path::PathBuf,
        loader: F,
    ) -> Result<FileMetadata, DomainError>
    where
        F: FnOnce() -> Result<FileMetadata, std::io::Error>,
    {
        self.metadata_cache.get_or_insert(path, loader).await
    }

    /// Verifica o determina si un archivo/carpeta existe
    #[allow(dead_code)]
    pub async fn check_exists<F>(
        &self,
        path: std::path::PathBuf,
        checker: F,
    ) -> Result<bool, DomainError>
    where
        F: FnOnce() -> Result<bool, std::io::Error>,
    {
        self.existence_cache.get_or_insert(path, checker).await
    }

    /// Invalida la caché para una ruta específica
    #[allow(dead_code)]
    pub async fn invalidate_path(&self, path: &std::path::Path) {
        self.metadata_cache.invalidate(&path.to_path_buf()).await;
        self.existence_cache.invalidate(&path.to_path_buf()).await;
    }

    /// Limpia todas las entradas expiradas
    #[allow(dead_code)]
    pub async fn cleanup(&self) -> (usize, usize) {
        let metadata_cleaned = self.metadata_cache.cleanup_expired().await;
        let existence_cleaned = self.existence_cache.cleanup_expired().await;
        (metadata_cleaned, existence_cleaned)
    }
}
