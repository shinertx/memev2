
# **🚀 COMPLETE MEMESNIPE v18 - "THE ALPHA ENGINE"**

## **📁 Project Structure**

```
meme-snipe-v18/
├── .env.example
├── .gitignore
├── docker-compose.yml
├── executor/
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── database.rs
│       ├── executor.rs
│       ├── jito_client.rs
│       ├── jupiter.rs
│       ├── portfolio_monitor.rs
│       ├── signer_client.rs
│       └── strategies/
│           ├── mod.rs
│           ├── airdrop_rotation.rs
│           ├── bridge_inflow.rs
│           ├── dev_wallet_drain.rs
│           ├── korean_time_burst.rs
│           ├── liquidity_migration.rs
│           ├── mean_revert_1h.rs
│           ├── momentum_5m.rs
│           ├── perp_basis_arb.rs
│           ├── rug_pull_sniffer.rs
│           └── social_buzz.rs
├── signer/
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       └── main.rs
├── shared-models/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── strategy_factory/
│   ├── Dockerfile
│   ├── factory.py
│   └── requirements.txt
├── meta_allocator/
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       └── main.rs
├── data_consumers/
│   ├── Dockerfile
│   ├── requirements.txt
│   ├── bridge_consumer.py
│   ├── depth_consumer.py
│   ├── funding_consumer.py
│   ├── helius_rpc_price_consumer.py
│   └── onchain_consumer.py  <-- NEW (for OnChain events)
├── position_manager/  <-- NEW SERVICE
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── database.rs
│       ├── jupiter.rs
│       ├── signer_client.rs
│       └── position_monitor.rs
├── dashboard/
│   ├── requirements.txt
│   ├── Dockerfile
│   ├── app.py
│   └── templates/
│       └── index.html
├── docs/
│   └── STRATEGY_TEMPLATE.md
├── prometheus.yml
└── scripts/
    └── deploy_vm_gcp.sh
```

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

## 🔧 **Operational Guide**

### **1. Initial Repository Setup (Local Machine)**

1.  **Clone the Repository:**
    ```bash
    git clone <your-repo-url>
    cd meme-snipe-v18 # Or whatever your cloned directory is named
    ```

2.  **Create Your Environment File:**
    ```bash
    cp .env.example .env
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

4.  **Configure `.env` for Paper Trading:**
    Open the `.env` file and fill in all placeholders. For initial paper trading, the `SOLANA_RPC_URL` and `JITO_RPC_URL` can point to devnet or mainnet public endpoints, but **API keys for data consumers are crucial even for paper trading with live data.**
    ```env
    PAPER_TRADING_MODE=true
    WALLET_KEYPAIR_FILENAME=my_wallet.json
    JITO_AUTH_KEYPAIR_FILENAME=jito_auth_key.json

    # --- Fill ALL API Keys ---
    SOLANA_RPC_URL=https://rpc.helius.xyz/?api-key=YOUR_HELIUS_API_KEY_HERE
    JITO_RPC_URL=https://mainnet.block-engine.jito.wtf/api
    # ... other API keys for data consumers (e.g., Pyth, Twitter, Drift)
    ```

### **2. Deploy & Verify Paper Trading (Local or GCP)**

1.  **Build Docker Images:**
    ```bash
    docker compose build
    ```
    *Expected Output:* All Docker images should build successfully.

2.  **Deploy the System (Paper Trading Mode):**
    ```bash
    docker compose up -d
    ```
    *Expected Output:* All services should be created and started.

3.  **Verify Service Health:**
    ```bash
    docker compose ps
    ```
    *Expected Output:* All services should show `running` status.

4.  **Monitor Logs (Initial Check):**
    ```bash
    docker compose logs -f executor meta_allocator strategy_factory data_consumers position_manager
    ```
    *Expected Output:*
    *   `data_consumers`: Should show logs of fetching and publishing *real* market data.
    *   `strategy_factory`: Publishing strategy specs (data simulation loop should be commented out for live data).
    *   `meta_allocator`: Calculating Sharpe ratios and publishing allocations.
    *   `executor`: Starting strategies, subscribing to event streams, and logging simulated `BUY`/`SELL` trades.
    *   `position_manager`: Monitoring open paper trades and simulating their closure.

5.  **Access the Dashboard:**
    Open your web browser and navigate to `http://localhost:8080`.
    *Expected Output:* The dashboard should populate with live data-driven paper trades, showing PnL, allocations, and strategy performance.

### **3. Go Live (Extreme Caution - Real Funds at Risk)**

**Proceed only after extensive, successful paper trading with live data and a thorough understanding of all risks.**

1.  **Final `.env` Change:**
    Open your `.env` file and change `PAPER_TRADING_MODE` to `false`.
    ```env
    PAPER_TRADING_MODE=false
    ```

2.  **Rebuild and Restart All Services:**
    ```bash
    docker compose build
    docker compose up -d
    ```
    *Expected Output:* All services will restart. The `executor` will now attempt to execute real trades on the Solana mainnet.

3.  **Intense Monitoring:**
    *   **Dashboard:** Keep the dashboard open and monitor global PnL, individual strategy performance, and trade history in real-time.
    *   **Logs:** Continuously tail the logs of the `executor`, `signer`, and `position_manager` services. Look for any errors, failed transactions, or unexpected behavior.
        ```bash
        docker compose logs -f executor signer position_manager
        ```
    *   **Wallet:** Monitor your actual Solana wallet balance using a block explorer (e.g., Solana Explorer) to confirm trades are executing and funds are moving as expected.
    *   **Alerts:** Set up external alerts (e.g., Prometheus Alertmanager, custom scripts) to notify you of critical events like failed trades, large drawdowns, or service outages.

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
