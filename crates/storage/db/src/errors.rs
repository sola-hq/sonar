// https://docs.rs/tracing-error/latest/tracing_error/
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Clickhouse error: {0}")]
    Clickhouse(#[from] clickhouse::error::Error),
}
