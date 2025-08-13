use anyhow::Result;
use socketioxide_redis::{
    drivers::redis::{redis_client::Client, RedisDriver},
    RedisAdapterCtr,
};
use std::env;

pub async fn init_adapter() -> Result<RedisAdapterCtr<RedisDriver>> {
    let redis_url = env::var("REDIS_ADAPTER_URL").expect("Expected REDIS_ADAPTER_URL to be set");
    let client = Client::open(redis_url)?;
    let adapter = RedisAdapterCtr::new_with_redis(&client).await?;
    Ok(adapter)
}
