use crate::db::{Database, DatabaseTrait};
use anyhow::Result;
use std::env::var;

pub mod db;
use db::ClickhouseDb;

/// Create a new Clickhouse database
///
/// # Arguments
///
/// * `database_url` - The URL of the Clickhouse database
/// * `user` - The username for the Clickhouse database
/// * `password` - The password for the Clickhouse database
/// * `database` - The name of the Clickhouse database
/// * `max_swap_event_rows` - The maximum number of swap events to store in the database,
///   defaults to 1000
/// * `max_token_rows` - The maximum number of tokens to store in the database,
///   defaults to 1, note that this is large than 1, the get tokens would return none,
///   please use it with caution
///
/// # Returns
///
/// A new Clickhouse database
pub async fn make_db(
    database_url: &str,
    user: &str,
    password: &str,
    database: &str,
    max_swap_event_rows: Option<u64>,
    max_token_rows: Option<u64>,
) -> Result<Database> {
    let max_swap_event_rows = max_swap_event_rows.unwrap_or(1000);
    let max_token_rows = max_token_rows.unwrap_or(1);
    let mut db = ClickhouseDb::new(database_url, user, password, database)
        .with_max_swap_event_rows(max_swap_event_rows)
        .with_max_token_rows(max_token_rows);
    db.initialize().await?;
    Ok(Box::new(db))
}

pub async fn make_db_from_env() -> Result<Database> {
    let database_url = var("CLICKHOUSE_URL").expect("Expected CLICKHOUSE_URL to be set");
    let user = var("CLICKHOUSE_USER").expect("Expected CLICKHOUSE_USER to be set");
    let password = var("CLICKHOUSE_PASSWORD").expect("Expected CLICKHOUSE_PASSWORD to be set");
    let database = var("CLICKHOUSE_DATABASE").expect("Expected CLICKHOUSE_DATABASE to be set");
    let max_swap_event_rows = var("CLICKHOUSE_MAX_SWAP_EVENTS_ROWS")
        .ok()
        .map(|v| v.parse::<u64>().expect("CLICKHOUSE_MAX_SWAP_EVENTS_ROWS must be a number"));
    let max_token_rows = var("CLICKHOUSE_MAX_TOKEN_ROWS")
        .ok()
        .map(|v| v.parse::<u64>().expect("CLICKHOUSE_MAX_TOKEN_ROWS must be a number"));
    make_db(&database_url, &user, &password, &database, max_swap_event_rows, max_token_rows).await
}
