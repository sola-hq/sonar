use crate::{
    db::DatabaseTrait,
    models::{
        candlesticks::Candlestick,
        swap::{SwapEvent, Trade},
        tokens::{TokenDailyStat, TokenPrice, TokenSearch, TokenStat, TopToken},
        Token,
    },
    CandlestickInterval,
};
use anyhow::{Context, Result};
use chrono::DateTime;
use clickhouse::{inserter::Inserter, Client};
use futures::future;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

pub struct ClickhouseDb {
    client: Client,
    is_initialized: bool,
    max_swap_event_rows: u64,
    swap_event_inserter: Option<Arc<RwLock<Inserter<SwapEvent>>>>,
    max_token_rows: u64,
    token_inserter: Option<Arc<RwLock<Inserter<Token>>>>,
}

impl ClickhouseDb {
    /// create_inserter creates an inserter for the swap event table
    fn create_swap_event_inserter(&self) -> Result<Inserter<SwapEvent>> {
        let inserter = self
            .client
            .inserter::<SwapEvent>("swap_events")
            .context("failed to prepare swap event insert statement")?
            .with_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(20)))
            .with_max_rows(self.max_swap_event_rows)
            .with_max_bytes(1_000_000) // swap event is roughly ~200 bytes
            .with_period(Some(Duration::from_secs(15)));
        Ok(inserter)
    }

    /// set the max rows for the inserter
    pub fn with_max_swap_event_rows(mut self, max_rows: u64) -> Self {
        self.max_swap_event_rows = max_rows;
        self
    }

    pub fn create_token_inserter(&self) -> Result<Inserter<Token>> {
        let inserter = self
            .client
            .inserter::<Token>("tokens")
            .context("failed to prepare token insert statement")?
            .with_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(20)))
            .with_max_rows(self.max_token_rows)
            .with_max_bytes(1_000) // token is roughly ~210 bytes
            .with_period(Some(Duration::from_secs(3)));

        Ok(inserter)
    }

    pub fn with_max_token_rows(mut self, max_rows: u64) -> Self {
        self.max_token_rows = max_rows;
        self
    }
}

#[async_trait::async_trait]
impl DatabaseTrait for ClickhouseDb {
    fn new(database_url: &str, user: &str, password: &str, database: &str) -> Self {
        let client = Client::default()
            .with_url(database_url)
            .with_user(user)
            .with_password(password)
            .with_database(database);

        info!("Connecting to ClickHouse at {}", database_url);
        Self {
            client,
            is_initialized: false,
            max_swap_event_rows: 1_000,
            swap_event_inserter: None,
            max_token_rows: 1,
            token_inserter: None,
        }
    }

    /// health_check checks the health of the clickhouse database
    async fn health_check(&self) -> Result<()> {
        debug!("clickhouse healthz");
        self.client
            .query("SELECT 1")
            .execute()
            .await
            .context("Failed to execute health check query")?;
        Ok(())
    }

    /// initialize initializes the clickhouse database
    async fn initialize(&mut self) -> Result<()> {
        debug!("initializing clickhouse");

        let swap_event_inserter = self.create_swap_event_inserter()?;
        let swap_event_inserter = Arc::new(RwLock::new(swap_event_inserter));
        self.swap_event_inserter = Some(swap_event_inserter);

        let token_inserter = self.create_token_inserter()?;
        let token_inserter = Arc::new(RwLock::new(token_inserter));
        self.token_inserter = Some(token_inserter);

        self.is_initialized = true;

        Ok(())
    }

    /// insert_swap_event uses a batched writer to avoid spamming writes
    /// it is configurable at the initializer
    async fn insert_swap_event(&self, swap_event: &SwapEvent) -> Result<()> {
        debug!("inserting swap event: {}", swap_event.signature);

        let mut inserter =
            self.swap_event_inserter.as_ref().expect("inserter not initialized").write().await;

        inserter.write(swap_event).context("Failed to write price to insert buffer")?;

        let pending = inserter.pending();
        debug!("Pending: {} rows ({} bytes)", pending.rows, pending.bytes);

        let stats = inserter.commit().await?;
        if stats.transactions > 0 {
            info!(
                "Committed {} swap events {} bytes in {} transactions",
                stats.rows, stats.bytes, stats.transactions
            );
        }
        Ok(())
    }

    /// get_candlesticks_by_token returns a list of candlesticks for a given token and interval
    #[instrument(skip(self))]
    async fn get_candlesticks_by_token(
        &self,
        mint: &str,
        pairs: &[String],
        interval: CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
    ) -> Result<Vec<Candlestick>> {
        let interval_seconds = interval.get_seconds();
        let limit = limit.unwrap_or(200);
        let mut conditions = vec![format!("pubkey = '{}'", mint)];

        if let Some(time_from) = time_from {
            conditions.push(format!("timestamp >= {}", time_from));
        }
        if let Some(time_to) = time_to {
            conditions.push(format!("timestamp < {}", time_to));
        }
        if !pairs.is_empty() {
            let placeholders = vec!["?"; pairs.len()].join(",");
            conditions.push(format!("pair IN ({})", placeholders));
        }

        let query = format!(
            r#"
            WITH 
                quantileExactWeighted(0.995)(price, 1) AS price_upper_bound, 
                quantileExactWeighted(0.005)(price, 1) AS price_lower_bound
            SELECT
                intDiv(timestamp, {interval_seconds}) * {interval_seconds} as bucket,
                argMin(price, timestamp) as open,
                if(max(price) > price_upper_bound * 20, price_upper_bound, max(price)) AS high, 
                if(min(price) < price_lower_bound / 20, price_lower_bound, min(price)) AS low, 
                argMax(price, timestamp) as close,
                sum(base_amount) as volume,
                sum(swap_amount) as turnover
            FROM swap_events
            WHERE {conditions}
            GROUP BY bucket
            ORDER BY bucket DESC
            LIMIT {limit}
            "#,
            conditions = conditions.join(" AND "),
            limit = limit
        );

        let mut query_builder = self.client.query(&query);
        if !pairs.is_empty() {
            for pair in pairs {
                query_builder = query_builder.bind(pair);
            }
        }

        let result = query_builder.fetch_all::<(u64, f64, f64, f64, f64, f64, f64)>().await?;

        let candlesticks: Vec<Candlestick> = result
            .into_iter()
            .map(|(timestamp, open, high, low, close, volume, turnover)| Candlestick {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                turnover,
            })
            .collect();

        // Reverse the order of the candlesticks
        let candlesticks = candlesticks.into_iter().rev().collect();

        Ok(candlesticks)
    }

    /// get_candlesticks_by_pair returns a list of candlesticks for a given pair and interval
    #[instrument(skip(self))]
    async fn get_candlesticks_by_pair(
        &self,
        pair: &str,
        token: Option<&str>,
        interval: &CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
    ) -> Result<Vec<Candlestick>> {
        let size = limit.unwrap_or(200);
        let mut candlesticks = self
            .get_candlesticks_from_swap_events(
                pair,
                token,
                interval,
                Some(size),
                time_from,
                time_to,
            )
            .await?;
        if candlesticks.len() < size {
            let exclude_buckets = candlesticks.iter().map(|c| c.timestamp).collect::<Vec<_>>();
            let additional_candlesticks = self
                .get_candlesticks_from_candlesticks(
                    pair,
                    token,
                    interval,
                    Some(size - candlesticks.len()),
                    time_from,
                    time_to,
                    Some(exclude_buckets),
                )
                .await?;
            candlesticks = [additional_candlesticks, candlesticks].concat();
        }
        // sort by timestamp ascending
        candlesticks.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        // truncate to size
        candlesticks.truncate(size);

        Ok(candlesticks)
    }

    #[instrument(skip(self))]
    async fn get_candlesticks_from_swap_events(
        &self,
        pair: &str,
        token: Option<&str>,
        interval: &CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
    ) -> Result<Vec<Candlestick>> {
        let interval_seconds = interval.get_seconds();
        let pairs = pair.split(",").map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(",");
        let mut conditions = vec![format!("pair IN ({})", pairs)];
        if let Some(token) = token {
            conditions.push(format!("pubkey = '{}'", token));
        }
        if let Some(time_from) = time_from {
            conditions.push(format!("timestamp >= {}", time_from));
        }
        if let Some(time_to) = time_to {
            conditions.push(format!("timestamp < {}", time_to));
        }
        let query = format!(
            r#"
            SELECT
                intDiv(timestamp, {interval_seconds}) * {interval_seconds} as bucket,
                argMin(price, timestamp) as open,
                max(price) as high,
                min(price) as low,
                argMax(price, timestamp) as close,
                sum(base_amount) as volume,
                sum(swap_amount) as turnover
            FROM swap_events
            WHERE {conditions}
            GROUP BY bucket
            ORDER BY bucket DESC
            LIMIT {limit}
            "#,
            conditions = conditions.join(" AND "),
            interval_seconds = interval_seconds,
            limit = limit.unwrap_or(200)
        );
        debug!(
            query = %query,
            table = "swap_events",
            "Executing SQL query"
        );

        let result =
            self.client.query(&query).fetch_all::<(u64, f64, f64, f64, f64, f64, f64)>().await?;
        let candlesticks: Vec<Candlestick> = result
            .into_iter()
            .map(|(timestamp, open, high, low, close, volume, turnover)| Candlestick {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                turnover,
            })
            .collect();
        // Reverse the order of the candlesticks
        let candlesticks = candlesticks.into_iter().rev().collect();
        Ok(candlesticks)
    }

    #[instrument(skip(self))]
    async fn get_candlesticks_from_candlesticks(
        &self,
        pair: &str,
        token: Option<&str>,
        interval: &CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
        exclude_buckets: Option<Vec<u64>>,
    ) -> Result<Vec<Candlestick>> {
        let interval_seconds = interval.get_seconds();
        let candlestick_interval = interval.get_candlestick_interval();
        let pairs = pair.split(",").map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(",");
        let mut conditions = vec![format!("pair IN ({})", pairs)];
        if let Some(token) = token {
            conditions.push(format!("pubkey = '{}'", token));
        }
        if let Some(time_from) = time_from {
            conditions.push(format!("timestamp >= {}", time_from));
        }
        if let Some(time_to) = time_to {
            conditions.push(format!("timestamp < {}", time_to));
        }
        if let Some(exclude_buckets) = exclude_buckets {
            if !exclude_buckets.is_empty() {
                let buckets =
                    exclude_buckets.iter().map(|s| format!("{}", s)).collect::<Vec<_>>().join(",");
                conditions.push(format!("bucket NOT IN ({})", buckets));
            }
        }
        let query = format!(
            r#"
            SELECT
                intDiv(timestamp, {interval_seconds}) * {interval_seconds} as bucket,
                argMin(open, timestamp) as open,
                max(high) as high,
                min(low) as low,
                argMax(close, timestamp) as close,
                sum(volume) as volume,
                sum(turnover) as turnover
            FROM candlesticks
            WHERE {conditions} AND interval = {candlestick_interval}
            GROUP BY bucket
            ORDER BY bucket DESC
            LIMIT {limit}
            "#,
            conditions = conditions.join(" AND "),
            interval_seconds = interval_seconds,
            candlestick_interval = candlestick_interval,
            limit = limit.unwrap_or(200)
        );
        debug!(
            query = %query,
            table = "candlesticks",
            "Executing SQL query"
        );

        let result =
            self.client.query(&query).fetch_all::<(u64, f64, f64, f64, f64, f64, f64)>().await?;

        let candlesticks: Vec<Candlestick> = result
            .into_iter()
            .map(|(timestamp, open, high, low, close, volume, turnover)| Candlestick {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                turnover,
            })
            .collect();

        // Reverse the order of the candlesticks
        let candlesticks = candlesticks.into_iter().rev().collect();

        Ok(candlesticks)
    }

    #[instrument(skip(self))]
    async fn get_top_tokens(
        &self,
        limit: usize,
        start_time: u64,
        min_volume: Option<f64>,
        min_market_cap: Option<f64>,
        pumpfun: Option<bool>,
    ) -> Result<Vec<TopToken>> {
        let mut query = format!(
            r#"
            WITH 
                latest_prices AS (
                    SELECT
                        pubkey,
                        price,
                        market_cap,
                        timestamp,
                        is_pump
                    FROM swap_events
                    WHERE timestamp >= {start_time}
                    ORDER BY timestamp DESC
                    LIMIT 1 BY pubkey
                ),
                volumes AS (
                    SELECT
                        pubkey,
                        sum(base_amount) as volume,
                        sum(swap_amount) as turnover
                    FROM swap_events
                    WHERE timestamp >= {start_time}
                    GROUP BY pubkey
                ),
                price_changes AS (
                    SELECT
                        pubkey,
                        (last_value(price) - first_value(price)) / first_value(price) * 100 as price_change
                    FROM swap_events
                    WHERE timestamp >= {start_time}
                    GROUP BY pubkey
                )
            SELECT
                lp.pubkey,
                lp.price,
                lp.market_cap,
                v.volume,
                v.turnover,
                pc.price_change
            FROM latest_prices lp
            LEFT JOIN volumes v ON lp.pubkey = v.pubkey
            LEFT JOIN price_changes pc ON lp.pubkey = pc.pubkey
            "#
        );

        let mut conditions = Vec::new();

        if let Some(min_volume) = min_volume {
            conditions.push(format!("v.volume >= {min_volume}"));
        }

        if let Some(min_market_cap) = min_market_cap {
            conditions.push(format!("lp.market_cap >= {min_market_cap}"));
        }

        if let Some(pumpfun) = pumpfun {
            conditions.push(format!("is_pump = {}", pumpfun));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(&format!(" ORDER BY v.volume DESC LIMIT {}", limit));
        let result = self.client.query(&query).fetch_all::<TopToken>().await?;
        Ok(result)
    }

    /// get_token_stats returns a list of token stats for a given list of tokens
    #[instrument(skip(self))]
    async fn get_token_stats(&self, mints: Vec<String>) -> Result<Vec<TokenStat>> {
        let query = r#"
            WITH 
                now() AS current_time, 
                toUnixTimestamp(current_time) AS current_ts 

            SELECT 
                pubkey,
                argMax(price, timestamp) AS latest_price, 
                argMax(market_cap, timestamp) AS latest_market_cap,

                coalesce(
                    NULLIF(argMax(price, timestamp) FILTER(WHERE timestamp <= current_ts - 300), 0.0), 
                    argMin(price, timestamp) FILTER(WHERE timestamp > current_ts - 300)
                ) AS price_5m,

                coalesce(
                    NULLIF(argMax(price, timestamp) FILTER(WHERE timestamp <= current_ts - 3600), 0.0), 
                    argMin(price, timestamp) FILTER(WHERE timestamp > current_ts - 3600)
                ) AS price_1h,

                coalesce(
                    NULLIF(argMax(price, timestamp) FILTER(WHERE timestamp <= current_ts - 21600), 0.0), 
                    argMin(price, timestamp) FILTER(WHERE timestamp > current_ts - 21600)
                ) AS price_6h,

                coalesce(
                    NULLIF(argMax(price, timestamp) FILTER(WHERE timestamp <= current_ts - 86400), 0.0), 
                    argMin(price, timestamp) FILTER(WHERE timestamp > current_ts - 86400)
                ) AS price_24h,

                sum(base_amount) FILTER(WHERE timestamp >= current_ts - 300) AS volume_5m,
                sum(base_amount) FILTER(WHERE timestamp >= current_ts - 3600) AS volume_1h,
                sum(base_amount) FILTER(WHERE timestamp >= current_ts - 21600) AS volume_6h,
                sum(base_amount) FILTER(WHERE timestamp >= current_ts - 86400) AS volume_24h,

                sum(swap_amount) FILTER(WHERE timestamp >= current_ts - 300) AS turnover_5m,
                sum(swap_amount) FILTER(WHERE timestamp >= current_ts - 3600) AS turnover_1h,
                sum(swap_amount) FILTER(WHERE timestamp >= current_ts - 21600) AS turnover_6h,
                sum(swap_amount) FILTER(WHERE timestamp >= current_ts - 86400) AS turnover_24h
            FROM swap_events
            WHERE pubkey IN ?
            GROUP BY pubkey
            "#;
        let result = self.client.query(query).bind(mints.clone()).fetch_all::<TokenStat>().await?;
        Ok(result)
    }

    /// get_token_daily_stats returns a list of token daily stats for a given list of tokens
    #[instrument(skip(self))]
    async fn get_token_daily_stats(&self, tokens: Vec<String>) -> Result<Vec<TokenDailyStat>> {
        let query = r#"
            SELECT 
                pubkey,
                end_ts as timestamp,
                latest_price as price,
                latest_market_cap as market_cap,
                price_24h,
                volume_24h,
                turnover_24h
            FROM token_24h_stats_v
            WHERE pubkey IN ? 
            "#;
        let result =
            self.client.query(query).bind(tokens.clone()).fetch_all::<TokenDailyStat>().await?;
        Ok(result)
    }

    /// get_trades returns a list of trades for a given query
    #[instrument(skip(self))]
    async fn get_trades(
        &self,
        address: Option<&str>,
        token: Option<&str>,
        pair: Option<&str>,
        signature: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Trade>> {
        let mut conditions = vec![];
        if let Some(pair) = pair {
            conditions.push(format!("pair = '{}'", pair));
        }
        if let Some(token) = token {
            conditions.push(format!("pubkey = '{}'", token));
        }
        if let Some(address) = address {
            conditions.push(format!("has(signers, '{}')", address));
        }
        if let Some(signature) = signature {
            conditions.push(format!("signature = '{}'", signature));
            conditions.push("timestamp >= toUnixTimestamp(now() - INTERVAL 1 HOUR)".to_string());
        }
        if conditions.is_empty() {
            return Ok(vec![]);
        }
        let query = format!(
            r#"
            SELECT
                pair,
                pubkey,
                price,
                market_cap,
                base_amount,
                quote_amount,
                swap_amount,
                owner,
                signature,
                signers,
                slot,
                timestamp,
                is_buy,
                is_pump
            FROM swap_events
            WHERE {cond}
            ORDER BY timestamp DESC
            LIMIT {limit} OFFSET {offset}
        "#,
            cond = conditions.join(" AND "),
            limit = limit.unwrap_or(100),
            offset = offset.unwrap_or(0),
        );
        let result = self.client.query(&query).fetch_all::<Trade>().await?;
        Ok(result)
    }

    /// get_price returns the price of a given mint at a given timestamp
    #[instrument(skip(self))]
    async fn get_price(&self, token: &str, timestamp: i32) -> Result<TokenPrice> {
        let token = token.to_string();
        let query = format!(
            r#"
            SELECT
                price,
                timestamp
            FROM swap_events
            WHERE pubkey = '{}' AND timestamp <= {}
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            token, timestamp
        );
        let result = self.client.query(&query).fetch_optional::<(f64, i32)>().await?;
        let price = match result {
            Some((price, neatest_timestamp)) => TokenPrice {
                token,
                timestamp,
                price: Some(price),
                neatest_timestamp: Some(neatest_timestamp),
            },
            None => TokenPrice { token, price: None, timestamp, neatest_timestamp: None },
        };
        Ok(price)
    }

    /// get_prices returns prices for multiple mint and timestamp combinations
    #[instrument(skip(self))]
    async fn get_prices(&self, tokens: Vec<(&str, i32)>) -> Result<Vec<TokenPrice>> {
        let tasks = tokens.into_iter().map(|(mint, timestamp)| self.get_price(mint, timestamp));
        let results = future::try_join_all(tasks).await?;
        Ok(results)
    }

    /// insert_token inserts a token into the database
    #[instrument(skip(self))]
    async fn insert_token(&self, token: &Token) -> Result<()> {
        let mut inserter =
            self.token_inserter.as_ref().expect("token inserter not initialized").write().await;
        inserter.write(token)?;

        let pending = inserter.pending();
        debug!("Pending: {} rows ({} bytes)", pending.rows, pending.bytes);

        let stats = inserter.commit().await?;
        if stats.rows > 0 {
            debug!(
                "Committed {} tokens {} bytes in {} transactions",
                stats.rows, stats.bytes, stats.transactions
            );
        }

        Ok(())
    }

    /// get_token returns a token from the database
    // #[instrument(skip(self))] skip because it's called in multiple places
    async fn get_token(&self, token: &str) -> Result<Option<Token>> {
        let query = format!(
            r#"
            SELECT * FROM tokens WHERE token = '{}' LIMIT 1
            "#,
            token
        );
        let result = self.client.query(&query).fetch_optional::<Token>().await?;
        Ok(result)
    }

    /// get_tokens returns a list of tokens from the database
    #[instrument(skip(self))]
    async fn get_tokens(&self, tokens: &[&str]) -> Result<Vec<Token>> {
        let addrs = tokens.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(",");
        let query = format!(
            r#"
            SELECT * FROM tokens WHERE token IN ({})
            "#,
            addrs
        );
        let result = self.client.query(&query).fetch_all::<Token>().await?;
        Ok(result)
    }

    /// has_token returns true if a token exists in the database
    async fn has_token(&self, token: &str) -> Result<bool> {
        let query = format!(
            r#"
            SELECT COUNT(*) FROM tokens WHERE token = '{}'
            "#,
            token
        );
        let result = self.client.query(&query).fetch_optional::<u64>().await?;
        Ok(result.is_some())
    }

    /// search_tokens returns a list of tokens that match a given query
    #[instrument(skip(self))]
    async fn search_tokens(&self, text: &str) -> Result<Vec<TokenSearch>> {
        let query = format!(
            r#"
            SELECT 
                token, name, symbol, decimals, supply, latest_price, price_24h, tx_count_24h, volume_24h, turnover_24h
            FROM token_search_with_stats_v 
            WHERE token = '{}' OR symbol ILIKE '%{}' OR symbol ILIKE '{}%' OR name ILIKE '%{}' OR name ILIKE '{}%'
            ORDER BY turnover_24h DESC
            LIMIT 10
            "#,
            text, text, text, text, text,
        );
        debug!(
            query = %query,
            table = "token_search_with_stats_v",
            "Executing SQL query"
        );

        let result = self.client.query(&query).fetch_all::<TokenSearch>().await?;
        Ok(result)
    }

    /// aggregate_into_candlesticks aggregates swap events into candlesticks table
    async fn aggregate_into_candlesticks(
        &self,
        start_time: i64,
        end_time: i64,
        interval: CandlestickInterval,
    ) -> Result<()> {
        let interval_seconds = interval.get_seconds();
        let query = format!(
            r#"
            INSERT INTO candlesticks
            SELECT
                pair,
                pubkey,
                {interval_seconds} as interval,
                intDiv(timestamp, {interval_seconds}) * {interval_seconds} as tp,
                argMin(price, timestamp) as open,
                max(price) as high,
                min(price) as low,
                argMax(price, timestamp) as close,
                sum(base_amount) as volume,
                sum(swap_amount) as turnover
            FROM swap_events
            WHERE timestamp >= {start_time} AND timestamp < {end_time}
            GROUP BY pubkey, pair, tp
            "#,
            interval_seconds = interval_seconds,
            start_time = start_time,
            end_time = end_time
        );
        self.client.query(&query).execute().await?;
        Ok(())
    }

    /// remove_swap_events removes swap events from the database
    async fn remove_swap_events(&self, timestamp: i64) -> Result<()> {
        let dt =
            DateTime::from_timestamp(timestamp, 0).context("Failed to create UTC timestamp")?;
        let yyyymmdd = dt.format("%Y%m%d").to_string();
        let query: String = format!("ALTER TABLE swap_events DROP PARTITION {}", yyyymmdd);
        debug!(query = %query, "Removing swap events from partition");
        self.client.query(&query).execute().await?;
        debug!("Removed swap events from partition: {}", yyyymmdd);
        Ok(())
    }
}
