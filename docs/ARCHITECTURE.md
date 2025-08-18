# Sonar Architecture Deep Dive

This document provides a more detailed explanation of the Sonar system architecture, expanding on the diagram in the main `README.md`.

## Core Principles

The architecture is designed around a few core principles:

-   **Modularity & Separation of Concerns**: Each major component (ingestion, scheduling, API) is a separate, independent service. This allows them to be developed, deployed, and scaled independently.
-   **Message-Driven**: Services are loosely coupled and communicate asynchronously via a message bus (Redis Pub/Sub). This makes the system resilient; a temporary failure in one service does not immediately bring down the entire system.
-   **Scalability**: The stateless nature of most services allows for horizontal scaling. For example, you could run multiple instances of the API server behind a load balancer to handle high traffic.

---

## Service Responsibilities

### 1. Ingestor (`crates/ingestor`)

The Ingestor is the heart of the system, acting as the primary entry point for all on-chain data.

-   **Primary Responsibility**: To connect to Solana data sources, parse raw transaction data, and transform it into structured, usable formats.
-   **Data Sources**: It primarily connects to a Solana node's Geyser plugin for the lowest latency. It can also use other sources like Helius or standard RPC endpoints.
-   **Functionality**:
    1.  **Filtering**: It listens to all account updates from Geyser but filters only for transactions involving specific programs (e.g., Raydium, Orca liquidity pools).
    2.  **Decoding**: It uses a suite of decoders (the `carbon-*` crates) to parse the raw instruction data of a transaction into a human-readable format (e.g., identifying a transaction as a `Swap` with specific input and output tokens and amounts).
    3.  **Enriching**: It enriches the decoded data with additional context, such as calculating the USD value of a swap by fetching the latest SOL/USD price from the Redis cache.
-   **Output / Data Flow**:
    -   **To ClickHouse**: It writes the final, processed event data (e.g., individual swaps) to a historical database (`swaps` table in ClickHouse) for long-term storage and analytics.
    -   **To Redis Pub/Sub**: It publishes real-time events (e.g., `new_swap`, `price_update`) to specific Redis channels. This is for services that need to react instantly.
    -   **To Redis Cache**: It may update real-time information, like the last traded price of a token pair, in the Redis cache for quick lookups by other services.

### 2. Scheduler (`crates/scheduler`)

The Scheduler handles all time-based, periodic, and background tasks that are not part of the real-time ingestion flow.

-   **Primary Responsibility**: To perform data aggregation and maintenance tasks on a recurring schedule.
-   **Functionality**:
    -   **Candlestick Aggregation**: This is its main job. For example, every minute, it queries the `swaps` table in ClickHouse for all trades that occurred in the last minute, and then calculates and stores the OHLCV (Open, High, Low, Close, Volume) data in a separate `candlesticks` table.
    -   **Data Health Checks**: It could be configured to periodically check data integrity or backfill missing data.

### 3. API Server (`crates/api`)

The API Server is the primary interface for end-users to access the data collected by Sonar.

-   **Primary Responsibility**: To serve both real-time and historical data via a user-friendly API.
-   **Functionality**:
    -   **REST API**: Provides access to historical data stored in ClickHouse. This includes endpoints for:
        -   Fetching historical trades for a given market.
        -   Fetching candlestick (OHLCV) data with various resolutions (e.g., 1m, 5m, 1h).
        -   Querying token information and other metadata.
    -   **WebSocket API**: Provides access to real-time data streams.
        -   It subscribes to the Redis Pub/Sub channels that the `Ingestor` publishes to.
        -   When a new message (e.g., a `new_swap` event) arrives from Redis, the API server immediately forwards it to all subscribed WebSocket clients who are listening for that specific market.

### 4. SOL Price Service (`crates/sol-price`)

This is a specialized utility service to ensure a reliable price feed for the native token (SOL), which is often the base pair for many calculations.

-   **Primary Responsibility**: To provide a canonical, real-time SOL/USD price.
-   **Functionality**: It connects to external, reliable sources like major CEX APIs (e.g., Binance, Coinbase) to fetch the current SOL/USD price.
-   **Output / Data Flow**:
    -   It writes the fetched price to a well-known key in the global cache.
    -   This allows other services, like the `Ingestor`, to easily access a reliable USD price without making external API calls themselves.

---

## Data Flow Example: A Single Swap

1.  A user performs a swap on a Raydium liquidity pool.
2.  The transaction is finalized on the Solana blockchain.
3.  The Geyser plugin on a Solana node streams the transaction data to the **Ingestor**.
4.  The **Ingestor** identifies it as a Raydium swap, decodes the instruction data, and calculates the amounts.
5.  The **Ingestor** fetches the latest SOL/USD price from the Redis cache (which was placed there by the **sol-price** service) to calculate the swap's USD value.
6.  The **Ingestor** writes the full swap details to the `swaps` table in ClickHouse.
7.  The **Ingestor** publishes a `new_swap` event to a Redis Pub/Sub channel (e.g., `SOLUSD`).
8.  The **API Server**, which is subscribed to this channel, receives the event and pushes it to all connected WebSocket clients listening to the SOLUSD market.
9.  Later, the **Scheduler** runs its one-minute job, reads this swap from ClickHouse along with others, and updates the `candlesticks` table for the SOLUSD pair.
10. A user can now query the REST API to get the historical data for that one-minute candlestick.
