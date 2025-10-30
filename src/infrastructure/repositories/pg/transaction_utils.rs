use sqlx::{Error as SqlxError, Executor, PgPool, Postgres, Transaction};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Helper function to execute database operations in a transaction
/// Takes a database pool and a closure that will be executed within a transaction
/// The closure receives a transaction object that should be used for all database operations
/// If the closure returns an error, the transaction is rolled back
/// If the closure returns Ok, the transaction is committed
pub async fn with_transaction<F, T, E>(
    pool: &Arc<PgPool>,
    operation_name: &str,
    operation: F,
) -> Result<T, E>
where
    F: for<'c> FnOnce(
        &'c mut Transaction<'_, Postgres>,
    ) -> futures::future::BoxFuture<'c, Result<T, E>>,
    E: From<SqlxError> + std::fmt::Display,
{
    debug!("Starting database transaction for: {}", operation_name);

    // Begin transaction
    let mut tx = pool.begin().await.map_err(|e| {
        error!("Failed to begin transaction for {}: {}", operation_name, e);
        E::from(e)
    })?;

    // Execute the operation within the transaction
    match operation(&mut tx).await {
        Ok(result) => {
            // If operation succeeds, commit the transaction
            match tx.commit().await {
                Ok(_) => {
                    debug!("Transaction committed successfully for: {}", operation_name);
                    Ok(result)
                }
                Err(e) => {
                    error!("Failed to commit transaction for {}: {}", operation_name, e);
                    Err(E::from(e))
                }
            }
        }
        Err(e) => {
            // If operation fails, rollback the transaction
            if let Err(rollback_err) = tx.rollback().await {
                error!(
                    "Failed to rollback transaction for {}: {}",
                    operation_name, rollback_err
                );
                // Still return the original error
            } else {
                info!("Transaction rolled back for {}: {}", operation_name, e);
            }
            Err(e)
        }
    }
}

/// Variant that accepts a transaction isolation level
pub async fn with_transaction_isolation<F, T, E>(
    pool: &Arc<PgPool>,
    operation_name: &str,
    isolation_level: TransactionIsolationLevel,
    operation: F,
) -> Result<T, E>
where
    F: for<'c> FnOnce(
        &'c mut Transaction<'_, Postgres>,
    ) -> futures::future::BoxFuture<'c, Result<T, E>>,
    E: From<SqlxError> + std::fmt::Display,
{
    debug!(
        "Starting database transaction with isolation level {:?} for: {}",
        isolation_level, operation_name
    );

    // Begin transaction with specific isolation level
    let mut tx = pool.begin().await.map_err(|e| {
        error!("Failed to begin transaction for {}: {}", operation_name, e);
        E::from(e)
    })?;

    // Set isolation level
    tx.execute(
        &format!(
            "SET TRANSACTION ISOLATION LEVEL {}",
            isolation_level.to_string()
        )[..],
    )
    .await
    .map_err(|e| {
        error!(
            "Failed to set isolation level for {}: {}",
            operation_name, e
        );
        E::from(e)
    })?;

    // Execute the operation within the transaction
    match operation(&mut tx).await {
        Ok(result) => {
            // If operation succeeds, commit the transaction
            match tx.commit().await {
                Ok(_) => {
                    debug!("Transaction committed successfully for: {}", operation_name);
                    Ok(result)
                }
                Err(e) => {
                    error!("Failed to commit transaction for {}: {}", operation_name, e);
                    Err(E::from(e))
                }
            }
        }
        Err(e) => {
            // If operation fails, rollback the transaction
            if let Err(rollback_err) = tx.rollback().await {
                error!(
                    "Failed to rollback transaction for {}: {}",
                    operation_name, rollback_err
                );
                // Still return the original error
            } else {
                info!("Transaction rolled back for {}: {}", operation_name, e);
            }
            Err(e)
        }
    }
}

/// Transaction isolation levels from SQL standard
#[derive(Debug)]
pub enum TransactionIsolationLevel {
    /// Read committed isolation level
    ReadCommitted,
    /// Repeatable read isolation level
    RepeatableRead,
    /// Serializable isolation level
    Serializable,
}

impl ToString for TransactionIsolationLevel {
    fn to_string(&self) -> String {
        match self {
            TransactionIsolationLevel::ReadCommitted => "READ COMMITTED".to_string(),
            TransactionIsolationLevel::RepeatableRead => "REPEATABLE READ".to_string(),
            TransactionIsolationLevel::Serializable => "SERIALIZABLE".to_string(),
        }
    }
}
