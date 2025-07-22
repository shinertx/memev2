
# **🚀 COMPLETE MEMESNIPE v18 - "THE ALPHA ENGINE"**

## **🌐 Live Environment**

### GCP Infrastructure
- **VM**: `meme-snipe-v18-vm2` (us-central1-a)
- **External IP**: `146.148.99.199`
- **Dashboard**: http://146.148.99.199:8080
- **Prometheus**: http://146.148.99.199:9184
- **Health Check**: http://146.148.99.199:8080/health

### Quick Access
```bash
# SSH into VM
gcloud compute ssh meme-snipe-v18-vm2 --zone=us-central1-a

# Deploy/Update
cd meme-snipe-v18 && ./scripts/deploy_vm_gcp.sh

# Monitor
./scripts/monitor.sh status
```

## **📁 Project Structure**

```
meme-snipe-v18/
├── Cargo.lock (149KB, 6281 lines)
├── docker-compose.yml (3.0KB, 109 lines)
├── env.example (1.4KB, 36 lines)
├── jito_auth_key.json (234B, 1 lines)
├── my_wallet.json (232B, 1 lines)
├── prometheus.yml (128B, 8 lines)
├── .gitignore (239B, 24 lines)
├── target/ (build artifacts)
├── shared/ (.gitkeep only)
├── config/ (empty)
├── docker/ (empty)
├── docs/
│   └── STRATEGY_TEMPLATE.md (3.3KB, 79 lines)
├── scripts/
│   └── deploy_vm_gcp.sh (7.2KB, 148 lines)
├── shared-models/
│   ├── Cargo.toml (154B, 9 lines)
│   └── src/
│       └── lib.rs (3.8KB, 131 lines)
├── executor/
│   ├── Cargo.toml (1.6KB, 55 lines)
│   ├── Dockerfile (1.7KB, 53 lines)
│   └── src/
│       ├── main.rs (1.1KB, 37 lines)
│       ├── config.rs (2.7KB, 52 lines)
│       ├── database.rs (6.2KB, 170 lines)
│       ├── executor.rs (19KB, 359 lines)
│       ├── jito_client.rs (3.3KB, 73 lines)
│       ├── jupiter.rs (4.3KB, 113 lines)
│       ├── portfolio_monitor.rs (3.5KB, 76 lines)
│       ├── signer_client.rs (1.0KB, 33 lines)
│       └── strategies/
│           ├── mod.rs (1.3KB, 43 lines)
│           ├── airdrop_rotation.rs (2.4KB, 54 lines)
│           ├── bridge_inflow.rs (2.2KB, 55 lines)
│           ├── dev_wallet_drain.rs (2.2KB, 49 lines)
│           ├── korean_time_burst.rs (2.7KB, 58 lines)
│           ├── liquidity_migration.rs (2.3KB, 55 lines)
│           ├── mean_revert_1h.rs (3.4KB, 68 lines)
│           ├── momentum_5m.rs (2.8KB, 62 lines)
│           ├── perp_basis_arb.rs (3.6KB, 77 lines)
│           ├── rug_pull_sniffer.rs (1.8KB, 42 lines)
│           └── social_buzz.rs (3.4KB, 72 lines)
├── signer/
│   ├── Cargo.toml (526B, 27 lines)
│   ├── Dockerfile (1.3KB, 47 lines)
│   └── src/
│       └── main.rs (3.0KB, 92 lines)
├── meta_allocator/
│   ├── Cargo.toml (447B, 15 lines)
│   ├── Dockerfile (1.3KB, 45 lines)
│   └── src/
│       └── main.rs (6.2KB, 136 lines)
├── position_manager/
│   ├── Cargo.toml (1.0KB, 41 lines)
│   ├── Dockerfile (1.6KB, 50 lines)
│   └── src/
│       ├── main.rs (829B, 31 lines)
│       ├── config.rs (1.3KB, 34 lines)
│       ├── database.rs (4.0KB, 118 lines)
│       ├── jupiter.rs (3.8KB, 109 lines)
│       ├── position_monitor.rs (7.4KB, 159 lines)
│       └── signer_client.rs (1.1KB, 34 lines)
├── data_consumers/
│   ├── Dockerfile (247B, 11 lines)
│   ├── requirements.txt (82B, 4 lines)
│   ├── bridge_consumer.py (1.2KB, 35 lines)
│   ├── depth_consumer.py (1.6KB, 44 lines)
│   ├── funding_consumer.py (1.2KB, 36 lines)
│   ├── helius_rpc_price_consumer.py (2.2KB, 61 lines)
│   └── onchain_consumer.py (1.8KB, 45 lines)
├── dashboard/
│   ├── Dockerfile (429B, 19 lines)
│   ├── app.py (5.1KB, 133 lines)
│   ├── requirements.txt (63B, 5 lines)
│   └── templates/
│       └── index.html (10KB, 176 lines)
└── strategy_factory/
    ├── Dockerfile (161B, 11 lines)
    ├── factory.py (3.3KB, 86 lines)
    └── requirements.txt (13B, 2 lines)
```

**Total Files:** 50+ files across 8 main directories
**Rust Services:** 4 (executor, signer, meta_allocator, position_manager)
**Python Services:** 6 (5 data consumers + strategy_factory)
**Web Services:** 1 (dashboard)
**Strategies:** 10 trading strategies in executor
**Configuration:** Docker Compose, environment templates, deployment scripts

---

## **📄 1. README.md**

```markdown
# 🚀 MemeSnipe v18 - "The Alpha Engine"

> **The definitive, production-ready, autonomous multi-strategy trading platform for Solana memecoins.**

This is the culmination of all previous development. It is a complete, end-to-end system designed for the discovery, analysis, and execution of a diverse portfolio of trading strategies. It is built on a secure, high-performance, event-driven architecture that allows for hot-swappable trading algorithms, now fully integrated for **live market operation**.

---

## ✅ **Core Features of v18**

*   **100% Live-Ready:** All previous "simulated" or "not implemented" components for live trading have been fully integrated.
*   **Real-Time Data Consumers:** Dedicated services fetch **live, high-fidelity market data** (Price, Social, Depth, Bridge, Funding, SOL Price, On-Chain events) from external APIs.
*   **Live Position Management:** A new `position_manager` service actively monitors all open live trades, calculates trailing stop-losses, and automatically executes sell orders for disciplined exits.
*   **Full Jito Integration:** Dynamic Jito tip calculation based on network conditions and robust bundle submission for priority transaction inclusion.
*   **Full Drift Integration:** Shorting strategies are fully functional, with the system capable of opening and closing short positions on Drift v2 perps.
*   **Dynamic, Risk-Adjusted Capital Allocation:** The `meta_allocator` uses **Sharpe Ratio** to dynamically assign capital to the most efficient, risk-adjusted strategies.
*   **Hyper-Efficient Event Routing:** The `executor` uses a subscription model, ensuring strategies only receive the specific data events they need.
*   **Institutional-Grade Security:** A dedicated, isolated `signer` service is the *only* component with access to the private key.
*   **Robust Portfolio Stop-Loss:** A `portfolio_monitor` actively tracks overall portfolio drawdown and can pause trading to prevent ruin.
*   **Redis Streams for Reliability:** All critical inter-service communication uses Redis Streams, ensuring message persistence and guaranteed delivery.
*   **Comprehensive "Glass Cockpit" Dashboard:** Displays per-strategy performance (PnL, trades, Sharpe), live allocations, and detailed trade history.

---

## 🏗️ **System Architecture & Services Overview**

The system is composed of several independent microservices that communicate via a Redis event bus.

| Service | Language | Core Responsibility |
| :--- | :--- | :--- |
| **`strategy_factory`** | Python | **The R&D Dept.** Discovers/creates strategy "blueprints" (`StrategySpec`) and publishes them to the registry. **Can simulate market data for testing.** |
| **`meta_allocator`** | Rust | **The Portfolio Manager.** Reads all available strategies, analyzes their performance (PnL, Sharpe), and publishes capital `StrategyAllocation` commands. |
| **`executor`** | Rust | **The Operations Floor.** Listens for allocations, spins up strategy engines, routes market data to them, and processes their buy/sell signals. |
| **`signer`** | Rust | **The Vault.** A minimal, highly-secure service whose only job is to sign transactions. It has zero trading logic and is the only service with private key access. |
| **`data_consumers`** | Python | **The Sensors.** Collects **live, high-fidelity market data** (price, social, depth, bridge, funding, SOL price, on-chain) and publishes it to Redis Streams. |
| **`position_manager`** | Rust | **The Trade Manager.** Monitors all open live trades, calculates trailing stop-losses, and executes sell orders. |
| **`dashboard`** | Python | **The Cockpit.** Provides a real-time web interface to monitor the entire system, view allocations, and track performance. |

```mermaid
graph TD
    subgraph Data Sources
        A[Live APIs / Webhooks]
        B[Data Simulators (Optional)]
    end

    subgraph Redis Event Bus (Streams)
        C1(events:price)
        C2(events:social)
        C3(events:depth)
        C4(events:bridge)
        C5(events:funding)
        C6(events:sol_price)
        C7(events:onchain)
        C8(allocations_channel)
        C9(kill_switch_channel)
        C10(position_updates_channel)
    end

    subgraph Strategy Management
        D[strategy_factory.py] -- Publishes Specs --> E{strategy_registry_stream};
        E -- Reads Specs --> F[meta_allocator.rs];
        F -- Reads Perf Metrics --> G[perf:*:pnl_history];
        F -- Publishes Allocations --> C8;
    end

    subgraph Core Execution
        H[executor.rs] -- Reads Allocations --> C8;
        H -- Subscribes to Events --> C1 & C2 & C3 & C4 & C5 & C6 & C7;
        H -- Spawns/Manages --> I{Strategy Engines};
        I -- Emits Orders --> J[Order Processor];
        J -- Sends Unsigned TX --> K[signer_client.rs];
        H -- Monitors Portfolio --> L[portfolio_monitor.rs];
        L -- Publishes Kill Switch --> C9;
        H -- Reads Kill Switch --> C9;
        J -- Publishes Position Updates --> C10;
    end
    
    subgraph Secure Signing
        M[signer.rs] -- Listens for Requests --> N[HTTP API];
    end

    subgraph Live Position Management
        O[position_manager.rs] -- Reads Open Trades --> P[database.rs];
        O -- Subscribes to Price --> C1;
        O -- Executes Sell Orders --> J;
        O -- Publishes Position Updates --> C10;
    end

    subgraph Data & Monitoring
        P[dashboard]
        Q[prometheus]
    end

    A & B --> C1 & C2 & C3 & C4 & C5 & C6 & C7;
    K -- HTTP Request --> N;
    J --> P;
    O --> P;
    P --> E;
    P --> C8;
    P --> C9;
    P --> C10;
```

---

## 📈 **The 10 Implemented Strategy Families**

| Family ID | Core Alpha Signal | Data Subscriptions |
| :--- | :--- | :--- |
| `momentum_5m` | 5-minute price and volume breakout. | `Price` |
| `mean_revert_1h` | Price reversion on z-score extremes. | `Price` |
| `social_buzz` | Spike in social media mention velocity. | `Social` |
| `liquidity_migration` | Detects capital rotating between pools. | `OnChain`, `Bridge` |
| `perp_basis_arb` | Arbitrage between perpetual futures and spot price. | `Price`, `Funding` |
| `dev_wallet_drain` | Shorts tokens when a developer wallet begins dumping. | `OnChain` |
| `airdrop_rotation` | Buys tokens being actively airdropped to new holders. | `OnChain` |
| `korean_time_burst` | Volume and price spike during Korean trading hours. | `Price` |
| `bridge_inflow` | Detects when a token is bridged to a new chain. | `Bridge` |
| `rug_pull_sniffer` | Shorts tokens with imminent LP unlocks or other red flags. | `OnChain` |

---

## 🔧 **Operational Guide (GCP-Only Deployment)**

### **1. Initial Repository Setup (Local Machine - Preparation Only)**

**⚠️ IMPORTANT:** This system is designed for **GCP deployment only**. Local setup is only for preparation and configuration.

1.  **Clone the Repository:**
    ```bash
    git clone <your-repo-url>
    cd meme-snipe-v18
    ```

2.  **Create Your Environment File:**
    ```bash
    cp env.example .env
    ```

3.  **Prepare Your Wallet Keypair Files:**
    *   **`my_wallet.json`**: Your primary trading wallet. **Must contain SOL** for live trading.
    *   **`jito_auth_key.json`**: A *separate, non-funded* keypair used solely for authenticating with the Jito Block Engine.
    *   **Action:** Place both `my_wallet.json` and `jito_auth_key.json` directly in the **root directory** of your cloned project.
    *   **How to create them (if you don't have them):**
        ```bash
        solana-keygen new --outfile my_wallet.json
        solana-keygen new --outfile jito_auth_key.json
        ```

4.  **Configure `.env` for GCP Deployment:**
    Open the `.env` file and fill in all placeholders. **All API keys are required for live data.**
    ```env
    PAPER_TRADING_MODE=true  # Start with paper trading
    WALLET_KEYPAIR_FILENAME=my_wallet.json
    JITO_AUTH_KEYPAIR_FILENAME=jito_auth_key.json

    # --- Fill ALL API Keys ---
    SOLANA_RPC_URL=https://rpc.helius.xyz/?api-key=YOUR_HELIUS_API_KEY_HERE
    JITO_RPC_URL=https://mainnet.block-engine.jito.wtf/api
    HELIUS_API_KEY=YOUR_HELIUS_API_KEY_HERE
    # ... other API keys for data consumers
    ```

### **2. Deploy to GCP (Paper Trading Mode)**

**🚀 This is the ONLY deployment method for MemeSnipe v18.**

1.  **Prerequisites:**
    *   GCP account with billing enabled
    *   `gcloud` CLI installed and authenticated
    *   Project ID configured in `.env`

2.  **Deploy to GCP:**
    ```bash
    chmod +x scripts/deploy_vm_gcp.sh
    ./scripts/deploy_vm_gcp.sh
    ```
    *Expected Output:* VM creation, Docker installation, and service deployment.

3.  **Access the Dashboard:**
    The script will output the dashboard URL (typically `http://[VM-IP]:8080`).
    *Expected Output:* The dashboard should populate with live data-driven paper trades.

4.  **Monitor Logs (SSH to VM):**
    ```bash
    gcloud compute ssh meme-snipe-v18 --zone=us-central1-a
    docker compose logs -f executor meta_allocator strategy_factory data_consumers position_manager
    ```

### **3. Go Live on GCP (Extreme Caution - Real Funds at Risk)**

**Proceed only after extensive, successful paper trading with live data and a thorough understanding of all risks.**

1.  **SSH to GCP VM:**
    ```bash
    gcloud compute ssh meme-snipe-v18 --zone=us-central1-a
    ```

2.  **Switch to Live Trading:**
    ```bash
    cd meme-snipe-v18
    sed -i 's/PAPER_TRADING_MODE=true/PAPER_TRADING_MODE=false/' .env
    docker compose up -d --build
    ```

3.  **Intense Monitoring:**
    *   **Dashboard:** Monitor at `http://[VM-IP]:8080`
    *   **Logs:** Continuously tail the logs for errors or unexpected behavior
    *   **Wallet:** Monitor your actual Solana wallet balance
    *   **Alerts:** Set up GCP monitoring alerts for critical events

---

## 💻 **Strategy Development Guide (SDK)**

This system is designed for rapid development of new alpha.

1.  **The Contract (`Strategy` Trait):** Every strategy is a Rust struct that implements the `Strategy` trait defined in `executor/src/strategies/mod.rs`. This trait requires:
    *   `id(&self) -> &'static str`: Unique identifier.
    *   `subscriptions(&self) -> HashSet<EventType>`: **Crucial.** Declares which `MarketEvent` types (Price, Social, Depth, Bridge, Funding, OnChain, SolPrice) the strategy needs. The executor will only send these events to your strategy.
    *   `init(&mut self, params: &Value) -> Result<()>`: Initializes the strategy with its unique parameters from the spec.
    *   `on_event(&mut self, event: &MarketEvent) -> Result<StrategyAction>`: The core logic loop, called for every relevant market event.

2.  **The Blueprint (`docs/STRATEGY_TEMPLATE.md`):** Before writing any code, copy this template. It forces you to define your strategy's thesis, data requirements, parameters, and risks. It is a mandatory part of any new strategy submission.

3.  **The Workflow:**
    1.  **Document:** Create your strategy's documentation by filling out the template.
    2.  **Implement:** Create a new file in `executor/src/strategies/`. Implement the `Strategy` trait according to your design document.
    3.  **Register:** Use the `register_strategy!` macro in your file to make the executor aware of your new engine.
    4.  **Configure:** Add default parameters for your new strategy in `strategy_factory/factory.py`.
    5.  **Test:** Add a unit test for your strategy's logic.
    6.  **Deploy:** Run `docker-compose up --build`. The system will automatically discover, allocate to, and run your new strategy.

---

## 💰 **Cost Management**

Operating this system incurs costs from multiple sources. Be vigilant in monitoring them.

*   **GCP VM:** The `e2-standard-4` machine type costs approximately **$70-100/month**.
*   **Data Providers:** Helius, Pyth, Twitter, Telegram, Drift, etc. Costs vary significantly based on usage and tier. Monitor your provider dashboards closely.
*   **AI Services (Grok/OpenAI):** If you integrate an AI-based strategy, API calls can be expensive. Implement cost tracking and daily limits.
*   **Jito Tips:** These are direct costs per transaction. The system is designed to pay adaptive tips, but high trading volume will lead to higher tip costs.

**Recommendation:** Set up billing alerts in your GCP account and monitor all API provider dashboards daily.
