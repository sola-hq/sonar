# Sonar

<p align="center">
  <a href="https://github.com/sola-hq/sonar/actions/workflows/lint.yml">
    <img src="https://github.com/sola-hq/sonar/actions/workflows/lint.yml/badge.svg" alt="Lint Status">
  </a>
  <a href="https://github.com/sola-hq/sonar/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/sola-hq/sonar" alt="License">
  </a>
  <a href="https://github.com/sola-hq/sonar">
    <img src="https://img.shields.io/github/stars/sola-hq/sonar?style=social" alt="GitHub Stars">
  </a>
</p>

<h3 align="center">
  A high-performance, open-source data platform for the Solana ecosystem.
</h3>

---

Sonar is a modular indexing and analytics platform for the Solana blockchain. It does the heavy lifting of capturing, processing, and serving real-time and historical on-chain data, so you can focus on building your application.

## 🤔 Why Sonar?

Accessing reliable, low-latency, and comprehensive DeFi data on Solana is complex and expensive. Sonar solves this by providing a self-hosted, open-source solution that is both powerful and flexible.

**Who is this for?**

*   **DeFi Developers**: Build sophisticated applications on top of a solid data foundation.
*   **Data Analysts & Researchers**: Query rich, granular historical data to uncover market insights.
*   **Arbitrageurs & MEV Searchers**: Get a real-time firehose of on-chain events to feed your trading strategies.
*   **Infrastructure Providers**: Offer reliable data endpoints as a service to your users.

## ✨ Features

- **High-Performance Ingestion**: Connects directly to a Solana Geyser plugin for microsecond-level data capture.
- **Modular & Scalable**: Services for ingestion, API, and scheduling are independent crates, allowing them to be scaled separately.
- **Multi-Protocol DEX Support**: Includes processors for a wide range of DEX protocols on Solana (Raydium, Orca, Meteora, etc.).
- **Real-time & Historical Data**: Provides WebSocket streams for live data and a REST API for historical queries.
- **Extensible by Design**: Easily add support for new protocols or data types.

## 🚀 Getting Started

### Prerequisites

- **Rust**: See [rust-toolchain.toml](./rust-toolchain.toml) for the exact version.
- **Docker**: For running dependent services like Redis and ClickHouse.
- **Access to Solana Data**:
    - For full, low-latency performance: A Solana node with the [Geyser plugin](https://solana.com/docs/rpc/geyser-plugins).
    - For basic functionality and development: A standard Solana RPC endpoint.

### 1. Clone the Repository

```bash
git clone https://github.com/sola-hq/sonar.git
cd sonar
```

### 2. Configure Your Environment

Copy the example environment file. You will need to fill in the required values, especially the database and RPC endpoint URLs.

```bash
cp .env.example .env
```

### 3. Run with Docker Compose (Recommended)

For a simple, one-command setup of all services (including Redis and ClickHouse), we recommend using Docker Compose.

*(Note: A `docker-compose.yml` file is being developed and will be available soon. For now, please run the services manually as described below.)*

### 4. Manual Build & Run

Build the project in release mode:

```bash
cargo build --release
```

Each service can be run using the main `sonar` binary. You will need to run each command in a separate terminal.

**Start the API Server:**
```bash
./target/release/sonar api
```

**Start the Ingestor (Node):**
```bash
./target/release/sonar node
```

**Start the Scheduler:**
```bash
./target/release/sonar scheduler
```

## 🏗️ Architecture

The system is composed of several key services that communicate via a message queue (e.g., Redis).

```mermaid
graph TD
	subgraph "Data Sources"
		Geyser[Solana Geyser]
		WebSocket[Solana WebSocket]
		Helius[Helius Enhanced WebSocket]
		Binance[External APIs e.g., Binance]
		CoinGecko[External APIs e.g., CoinGecko]
	end

	subgraph "Sonar Services"
		Ingestor[Ingestor]
		Scheduler[Scheduler]
		API[API Server]
		SOLPRICE[sol-price]
	end

	subgraph "Infrastructure"
        MQ[(MQ-Redis)]
        CK[(ClickHouse)]
        CACHE[(Redis Cache)]
	end

	subgraph "Users"
		HTTP[HTTP Clients]
		WS[WebSocket Clients]
	end

	%% Data Sources to Services
	Geyser -->|transaction data| Ingestor
	WebSocket -->|transaction data| Ingestor
	Helius -->|transaction data| Ingestor
	Binance -->|SOL/USD price| SOLPRICE
	CoinGecko -->|token prices| SOLPRICE
	
	%% Services to Infrastructure
	Ingestor -->|token data, prices| CACHE
	Ingestor -->|price events| MQ
	Ingestor -->|processed events| CK
	Scheduler -->|read/write| CK
	API -->|read| CK
	API -->|consume events| MQ
	SOLPRICE -->|price updates| CACHE
	SOLPRICE -->|price events| MQ
	
	%% Users to Services
	HTTP -->|REST API| API
	WS -->|WebSocket API| API
	
	%% Styling
	classDef dataSource fill:#e1f5fe,stroke:#01579b,stroke-width:2px
	classDef service fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
	classDef infrastructure fill:#e8f5e8,stroke:#1b5e20,stroke-width:2px
	classDef user fill:#fff3e0,stroke:#e65100,stroke-width:2px
	
	class Geyser,WebSocket,Helius,Binance,CoinGecko dataSource
	class Ingestor,Scheduler,API,SOLPRICE service
	class MQ,CK,CACHE infrastructure
	class HTTP,WS user
```

## 🤝 Contributing

We welcome contributions of all kinds! A great way to start is by looking at our open issues. Please feel free to submit a pull request or open an issue.

For more detailed guidelines, please see our [CONTRIBUTING.md](./CONTRIBUTING.md) file (coming soon!).

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.
