-- swap events
CREATE TABLE IF NOT EXISTS swap_events (
  pair LowCardinality(String) CODEC(LZ4),
  pubkey LowCardinality(String) CODEC(LZ4),
  price Float64,
  market_cap Float64,
  timestamp UInt64,
  slot UInt64,
  base_amount Float64,
  quote_amount Float64,
  swap_amount Float64,
  owner LowCardinality(String) CODEC(LZ4),
  signature String CODEC(LZ4),
  signers Array(String) CODEC(LZ4),
  is_buy Bool,
  is_pump Bool,
  INDEX idx_pubkey_timestamp (pubkey, timestamp) TYPE minmax GRANULARITY 1,
  INDEX idx_signers signers TYPE bloom_filter(0.01) GRANULARITY 4,
  INDEX idx_signature_timestamp (signature, timestamp) TYPE minmax GRANULARITY 1024

  -- we could use projections, but it's not worth it for the current query
  -- PROJECTION projection_by_pubkey (SELECT pubkey, timestamp, price, market_cap, base_amount, quote_amount, swap_amount ORDER BY pubkey, timestamp),
	-- PROJECTION projection_by_owner (SELECT owner, timestamp, price, base_amount, quote_amount, swap_amount, signature ORDER BY owner, timestamp)
)
ENGINE = MergeTree() 
PARTITION BY toYYYYMMDD(fromUnixTimestamp(timestamp))
PRIMARY KEY (pair, pubkey, timestamp)
ORDER BY (pair, pubkey, timestamp);

-- Create view to get the final OHLCV data
CREATE VIEW IF NOT EXISTS swap_events_ohlcv_v AS
SELECT
    pair,
    pubkey,
    argMin(price, timestamp) as open,
    max(price) as high,
    min(price) as low,
    argMax(price, timestamp) as close,
    sum(base_amount) as volume,
    sum(swap_amount) as turnover,
    argMax(market_cap, timestamp) as market_cap,
    timestamp
FROM swap_events
GROUP BY pair, pubkey, timestamp;

-- create the candlestick table
CREATE TABLE IF NOT EXISTS candlesticks
(
    `pair` LowCardinality(String) CODEC(LZ4),
    `pubkey` LowCardinality(String) CODEC(LZ4),
    `interval` UInt32,
    `timestamp` UInt64,
    `open` Float64,
    `high` Float64,
    `low` Float64,
    `close` Float64,
    `volume` Float64,
    `turnover` Float64
)
ENGINE = ReplacingMergeTree(timestamp)
PARTITION BY toYYYYMMDD(fromUnixTimestamp(timestamp))
PRIMARY KEY (pubkey, pair, timestamp)
ORDER BY (pubkey, pair, timestamp);
