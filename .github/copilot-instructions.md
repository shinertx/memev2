# MemeSnipe v18 - AI Coding Instructions

# Overview
All code, refactors, and infrastructure changes must measurably increase net edge, reduce capital loss, or improve risk/risk-adjusted return—never the reverse.

No speculative, stylistic, or “just cleaner” changes are allowed if they degrade speed, stability, or measurable edge.

Multi-Role Perspective
You must reason like a Quantitative Researcher (signal decay, overfit defense), Quant Trader (execution, market structure), Risk Engineer (drawdown, fat-tail, circuit breaker), Rust SWE (async safety, zero panic, idiomatic), and SRE (reliability, rollback, chaos scenario).

Every PR or code suggestion should state: “What edge does this create, or what loss does it prevent?”

Strategy/Alpha-Specific Practices
No new strategy logic may ship without:

Explicit documentation of its alpha thesis, risk exposures, and failure modes (use docs/STRATEGY_TEMPLATE.md).

Backtest + out-of-sample walk-forward showing improvement in net Sharpe, drawdown, or PnL net of costs.

Shadow mode by default: New strategies run only in Paper until proven.

No cross-strategy data leakage: All live deployments are isolated.

Risk & Loss Minimization
Every trade flow must pass through mode checks, and risk guardrails.

All position sizing, stop-loss, and portfolio halts must be respected in all modes, with no bypass.

Every service must be restartable and safe to kill at any point (idempotent, no stuck state).

Operational/Infra Practices
All config/env changes must be idempotent, rollbackable, and versioned.

Redis Streams and DB schema changes must provide backward compatibility.

Any performance optimization must not break risk rails or cause new edge leaks.

Testing, Metrics, and Observability
All PRs must include unit tests for new logic, and regression tests for edge and risk metrics.

Add metrics or logs for any new code path that could impact risk, edge, or strategy state.

Peer/AI Review Guidance
Copilot or reviewers must explain not just “how” but “why” any suggestion increases edge or reduces loss.

Suggestions without explicit alpha/risk improvement rationales are to be rejected.



## Architecture Overview

MemeSnipe v18 is a production-ready autonomous trading system for Solana memecoins built as a microservices architecture with Rust and Python services communicating via Redis Streams. The system operates in two modes: **Paper Trading** (default/safe) and **Live Trading** (real money).

### Core Service Architecture (Unified)

```
                                     ┌───────────────────┐
                                     │  Observability    │
                                     │ (Grafana/Prometheus)│
                                     └─────────┬─────────┘
                                               │
Data Sources → Redis Streams → Strategy Execution → Risk Management → Transaction Signing → Trade Execution
     ↓              ↓                ↓                    ↓                    ↓                ↓
data_consumers → executor    →   risk_guardian   →   wallet_guard    →    signer      →  Jupiter/Jito
     └───────────────→ autonomous_allocator ←───────────────┘
```

**Key Services:**
- **executor** (Rust): Main orchestrator that runs trading strategies and processes market events.
- **autonomous_allocator** (Rust): ML-driven portfolio manager that allocates capital based on strategy performance.
- **risk_guardian** (Rust): Portfolio-wide risk controls and circuit breakers.
- **position_manager** (Rust): Monitors live trades and executes stop-losses.
- **wallet_guard** (Rust): Transaction security layer, enforcing limits before signing.
- **signer** (Rust): Isolated service with wallet access for transaction signing.
- **data_consumers** (Python): Fetches real-time market data from external APIs.
- **strategy_factory** (Python): Discovers and registers new trading strategies.
- **dashboard** (Python Flask): Web interface for monitoring system performance.
- **prometheus** (Docker): Collects metrics from all services.
- **grafana** (Docker): Visualizes metrics and system health.

## Development Patterns

### Canonical Docker Builds
All services are built using one of two universal Dockerfiles, ensuring consistency and maintainability.
- **`Dockerfile.rust`**: A multi-stage, performance-tuned (`jemalloc`) file for all Rust services.
- **`Dockerfile.python`**: A universal file for all Python services.
- **`docker-compose.yml`**: THE ONLY COMPOSE FILE - NO OTHER COMPOSE FILES SHOULD EXIST OR BE CREATED.

⚠️ **IMPORTANT**: If you see references to `docker-compose.prod.yml`, `docker-compose.dev.yml`, or any other compose variants, they are OUTDATED. Only `docker-compose.yml` exists.

### Strategy Development Workflow
1. **Copy `docs/STRATEGY_TEMPLATE.md`** - Every strategy starts with documentation.
2. **Implement in `executor/src/strategies/`** - Create new `.rs` file implementing `Strategy` trait.
3. **Use `register_strategy!` macro** - Register with the executor's dynamic loading system.
4. **Add to `strategy_factory/factory.py`** - Configure default parameters and risk profile.
5. **Add Metrics**: Expose new Prometheus metrics for strategy-specific performance.

### Essential Strategy Trait Implementation
```rust
#[async_trait]
pub trait Strategy {
    fn id(&self) -> &'static str;
    fn subscriptions(&self) -> HashSet<EventType>; // Crucial for event routing
    async fn init(&mut self, params: &Value) -> Result<()>;
    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction>;
}
```

### Inter-Service Communication via Redis Streams
- **Market Data**: `events:price`, `events:social`, `events:depth`, `events:bridge`, etc.
- **Strategy Lifecycle**: `strategy_registry_stream`, `allocations_channel`
- **Risk Management**: `kill_switch_channel`, `position_updates_channel`
- **Observability**: All services expose a `/metrics` endpoint for Prometheus.

## Key Development Commands

### Deployment & Operations (Unified)
The entire system is managed via a single, canonical Compose file.
```bash
# Build all services using the canonical Dockerfiles
docker compose build --parallel

# Deploy the entire stack (trading + observability)
docker compose up -d

# Check status of all services
docker compose ps

# View logs for a specific service
docker compose logs -f executor

# ⚠️ NEVER use -f flag with compose files, there is only ONE docker-compose.yml
```

### Configuration Management
- **Primary Config**: `docker-compose.yml` is the ONLY compose file.
- **Environment**: Copy `.env.example` → `.env` and configure API keys.
- **Wallet Setup**: Place `my_wallet.json` and `jito_auth_key.json` in project root.
- **Trading Mode**: Control via `PAPER_TRADING_MODE=true/false` in `.env`.
- **NO VARIANTS**: Do not create .prod, .dev, .test compose files - use environment variables instead.

### Database & State Management
- **SQLite**: Trade history and performance metrics in `shared/trades_v18.db`.
- **Redis**: Real-time event streams and inter-service messaging.
- **Prometheus**: Time-series database for all system metrics.

## Critical Safety Patterns

### Trading Mode Controls
- **Paper Trading**: Default mode, simulates trades without real money.
- **Live Trading**: Set `PAPER_TRADING_MODE=false` in `.env` and restart the `executor` and `position_manager`.
- **Mode Promotion**: `autonomous_allocator` can promote strategies from Paper → Live based on performance.

### Risk Management Integration
- **Portfolio Stop-Loss**: `risk_guardian` halts trading on drawdown limits.
- **Position Limits**: `wallet_guard` enforces per-trade size limits.
- **Circuit Breakers**: Emergency stop via `kill_switch_channel` Redis stream.

### Security Architecture
- **Isolated Signer**: Only the `signer` service has private key access.
- **Non-Root Containers**: All service containers run as a non-root `appuser` to minimize potential damage.
- **Transaction Flow**: `executor` → `risk_guardian` → `wallet_guard` → `signer` → blockchain.
- **Wallet Files**: Must be placed in project root, mounted read-only into the `signer` container.

## External Integration Points

### Market Data Sources
- **Helius**: Solana blockchain data via `data_consumers/helius_rpc_price_consumer.py`.
- **Jupiter**: DEX aggregation via `executor/src/jupiter.rs`.
- **Jito**: MEV protection via `executor/src/jito_client.rs`.
- **Drift**: Perpetual futures via integrated `drift-rs` library.

### Monitoring & Observability
- **Dashboard**: http://localhost (shows real-time PnL and trades)
- **Grafana**: http://localhost:3000 (for deep system monitoring, latency, PnL, etc.)
- **Prometheus**: http://localhost:9090 (for ad-hoc metric queries)
- **Health Checks**: All services have healthchecks defined in `docker-compose.prod.yml`.
- **Log Aggregation**: Centralized via Docker Compose logging config.

## Project-Specific Conventions

### Error Handling
- Use `anyhow::Result<T>` for all service operations.
- Strategy errors should return `StrategyAction::None` rather than panic.
- Database errors are logged but don't halt execution.

### Performance Optimization
- **Event Routing**: Strategies only receive subscribed `EventType`s via executor's filtering.
- **Async/Await**: All I/O operations use Tokio async runtime.
- **Connection Pooling**: Redis connections managed via `redis::ConnectionManager`.
- **`jemalloc`**: All Rust services are compiled with `jemalloc` for faster memory allocation.

### Testing Strategy
- **Unit Tests**: Per-strategy logic testing in `executor/src/strategies/`.
- **Integration Tests**: Full Docker Compose stack testing via `deploy.sh` (to be updated).
- **Paper Trading**: Extended testing phase before live trading promotion.

When working with this codebase, always use `docker compose` (no -f flag needed) as there is only one compose file. Verify trading mode before making changes that affect order execution, and understand the Redis Stream event flow when adding new data sources or strategies.

## Development Patterns

### Strategy Development Workflow
1. **Copy `docs/STRATEGY_TEMPLATE.md`** - Every strategy starts with documentation
2. **Implement in `executor/src/strategies/`** - Create new `.rs` file implementing `Strategy` trait
3. **Use `register_strategy!` macro** - Register with the executor's dynamic loading system
4. **Define data subscriptions** - Use `subscriptions()` method to declare required `EventType`s
5. **Add to `strategy_factory/factory.py`** - Configure default parameters

### Essential Strategy Trait Implementation
```rust
#[async_trait]
pub trait Strategy {
    fn id(&self) -> &'static str;
    fn subscriptions(&self) -> HashSet<EventType>; // Crucial for event routing
    async fn init(&mut self, params: &Value) -> Result<()>;
    async fn on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction>;
}
```

### Inter-Service Communication via Redis Streams
- **Market Data**: `events:price`, `events:social`, `events:depth`, `events:bridge`, etc.
- **Strategy Lifecycle**: `strategy_registry_stream`, `allocations_channel`
- **Risk Management**: `kill_switch_channel`, `position_updates_channel`
- Pattern: Producer publishes to stream, consumers use `XREAD` with consumer groups

## Key Development Commands

### Deployment & Operations
```bash
# Primary deployment (GCP-focused)
./deploy.sh deploy               # Full system deployment
./deploy.sh status              # Check all services
./deploy.sh logs [service]      # View specific service logs

# Development workflow
make test                       # Format, lint, and test all Rust code
make build                      # Build all Docker images
cargo test --all               # Run Rust tests only
```

### Configuration Management
- **Environment**: Copy `.env.example` → `.env` and configure API keys
- **Wallet Setup**: Place `my_wallet.json` and `jito_auth_key.json` in project root
- **Trading Mode**: Control via `PAPER_TRADING_MODE=true/false` in `.env`

### Database & State Management
- **SQLite**: Trade history and performance metrics in `shared/trades_v18.db`
- **Redis**: Real-time event streams and inter-service messaging
- **Performance Tracking**: Per-strategy PnL and Sharpe ratios stored with `perf:*:pnl_history` keys

## Critical Safety Patterns

### Trading Mode Controls
- **Paper Trading**: Default mode, simulates trades without real money
- **Live Trading**: Set `PAPER_TRADING_MODE=false` in `.env` and restart
- **Mode Promotion**: Autonomous Meallocator can promote strategies from Paper → Live based on performance

### Risk Management Integration
- **Portfolio Stop-Loss**: `portfolio_monitor.rs` halts trading on drawdown limits
- **Position Limits**: Per-strategy and global position size controls
- **Circuit Breakers**: Emergency stop via `kill_switch_channel` Redis stream

### Security Architecture
- **Isolated Signer**: Only the `signer` service has private key access
- **Transaction Flow**: `executor` → `signer_client.rs` → HTTP → `signer` → blockchain
- **Wallet Files**: Must be placed in project root, mounted read-only in Docker

## External Integration Points

### Market Data Sources
- **Helius**: Solana blockchain data via `data_consumers/helius_rpc_price_consumer.py`
- **Jupiter**: DEX aggregation via `executor/src/jupiter.rs`
- **Jito**: MEV protection via `executor/src/jito_client.rs`
- **Drift**: Perpetual futures via integrated `drift-rs` library

### Monitoring & Observability
- **Dashboard**: http://localhost (shows real-time performance)
- **Health Checks**: http://localhost/health
- **Prometheus Metrics**: Port 9090 on executor service
- **Log Aggregation**: Centralized via Docker Compose logging config

## Project-Specific Conventions

### Error Handling
- Use `anyhow::Result<T>` for all service operations
- Strategy errors should return `StrategyAction::None` rather than panic
- Database errors are logged but don't halt execution

### Performance Optimization
- **Event Routing**: Strategies only receive subscribed `EventType`s via executor's filtering
- **Async/Await**: All I/O operations use Tokio async runtime
- **Connection Pooling**: Redis connections managed via `redis::ConnectionManager`

### Testing Strategy
- **Unit Tests**: Per-strategy logic testing in `executor/src/strategies/`
- **Integration Tests**: Full Docker Compose stack testing via `deploy.sh`
- **Paper Trading**: Extended testing phase before live trading promotion

When working with this codebase, always use `docker compose` (no -f flag needed) as there is only one compose file. Verify trading mode before making changes that affect order execution, and understand the Redis Stream event flow when adding new data sources or strategies.
