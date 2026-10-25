# 🤖 Pravda-Bot

A Python cryptocurrency market-monitoring bot for Kraken and Binance. It estimates cross-exchange spreads, applies basic price and fee guards, and can ask a locally hosted model through LM Studio to analyze qualifying spreads.

> **Important:** This project uses public last-trade ticker data only and never places real orders. The paper simulator tracks virtual positions by default using these prices; estimates are not executable profit estimates, fills, or financial advice. `TRADING_MODE` is a display label and does not change execution behavior.

---

## 🚀 Key Features

* **Real-Time Multi-Exchange Monitoring**: Subscribes to public ticker WebSockets for BTC/USDT, ETH/USDT, and SOL/USDT on Kraken and Binance. It monitors last-trade prices, not order-book bid/ask prices.
* **Rich Dashboard**: Refreshes a compact per-symbol market and sampled-spread summary at a configurable interval instead of printing every ticker update.
* **Local Web Dashboard**: Serves recent spread observations from `spread_observations.csv` over localhost, with a configurable threshold and an observation freshness indicator.
* **Safety Guards**:
  * Rejects prices that are missing, non-finite, zero, or negative.
  * Estimates net spread by subtracting a fixed 0.36% fee estimate from the absolute gross spread.
  * Uses a configurable minimum gross spread threshold (default `0.05%`) and rejects invalid threshold settings at startup.
  * Skips LLM analysis when the estimated net spread is zero or negative.
  * Requires a candidate spread to remain profitable for a configurable confirmation interval and uses hysteresis before rearming alerts.
  * Rejects stale or excessively desynchronized exchange prices before spread evaluation.
* **Local LLM Analysis**: Sends qualifying symbol and price data to the configured `LM_STUDIO_API_BASE` chat-completions endpoint. Use a trusted local LM Studio endpoint if the data must remain on your network.
* **Price-Movement Alerts**: Detects ticker changes over WebSockets. ntfy notifications have a default five-second per-pair cooldown and a ten-second global send interval. Ticks suppressed by the global limit are not queued.
* **Arbitrage Alerts and Logging**: Sends a separate alert and logs an LLM analysis only when the estimated net spread becomes positive. `spread_observations.csv` records sampled numeric spread data (default every 10 seconds per pair), including threshold and fee values, for later offline analysis with `analyze_spreads.py`.
* **Paper Simulator**: Tracks a virtual portfolio or fixed-size positions without sending exchange orders. It estimates spread-capture P&L after configured fees and records closed simulations in `paper_trades.csv`; this is not a fill-accurate backtest.
* **Centralized Configuration Validation**: Validates endpoints, required model settings, and numeric intervals at startup with actionable errors.
* **No Order Placement**: The script does not use exchange API keys and contains no order-placement calls.

The 0.36% fee estimate is fixed and approximate. It does not account for slippage, withdrawal/network costs, latency, or exchange-specific fee tiers. The gross spread uses an absolute price difference; the paper simulator uses the signed difference between last prices to estimate spread capture, but does not model order books, fills, funding, borrowing, or exchange constraints. The threshold is configurable, not automatically optimized; use the observations as historical diagnostics rather than as evidence of executable profitability.

Notifications are disabled when `NTFY_TOPIC_URL` is unset. To enable them, configure a private, randomly generated topic URL in your local `.env`; never commit or share that topic. ntfy.sh topics are unlisted, not access-controlled, so anyone who learns the topic name may be able to read or publish notifications. After HTTP 429, sends pause according to `Retry-After` or an exponential backoff starting at 60 seconds (up to 15 minutes). `PRICE_ALERT_COOLDOWN_SECONDS=0` disables the per-pair cooldown but does not remove the global limit or server backoff.

---

## 🛠️ Tech Stack

* **Language**: Python 3.10+
* **Exchange Data**: Kraken and Binance public WebSocket ticker streams
* **AI / LLM**: LM Studio-compatible chat-completions API
* **Environment Configuration, WebSockets, Notifications and Terminal Output**: `python-dotenv`, `requests`, `websockets`, `rich`

---

## ⚙️ Installation & Quickstart

1. **Clone the repository**:
   ```bash
   git clone https://github.com/vincentdsb/Pravda-Bot.git
   cd Pravda-Bot
   ```

2. **Create and activate a virtual environment** (PowerShell):
   ```powershell
   py -m venv .venv
   .\.venv\Scripts\Activate.ps1
   ```

3. **Install dependencies**:
   ```powershell
   python -m pip install -r requirements.txt
   ```

4. **Start LM Studio** and load the desired model. Enable its local server and set the endpoint and model name for this shell:
   ```powershell
   $env:LM_STUDIO_API_BASE = "http://127.0.0.1:1234/v1"
   $env:LM_MODEL_NAME = "qwen/qwen3.5-9b"
   $env:TRADING_MODE = "paper"
   $env:MIN_SPREAD_THRESHOLD = "0.05"
   $env:SPREAD_LOG_INTERVAL_SECONDS = "10"
   $env:DASHBOARD_INTERVAL_SECONDS = "30"
   $env:ALERT_CONFIRMATION_SECONDS = "3"
   $env:ALERT_REARM_HYSTERESIS_PERCENT = "0.05"
   $env:MARKET_DATA_STALE_SECONDS = "15"
   $env:MARKET_DATA_MAX_SKEW_SECONDS = "3"
   $env:PAPER_MODE = "portfolio"
   $env:PAPER_STARTING_BALANCE_USDT = "1000"
   $env:PAPER_POSITION_PERCENT = "10"
   $env:PAPER_FIXED_POSITION_USDT = "100"
   $env:PAPER_EXIT_SPREAD_PERCENT = "0.05"
   $env:PAPER_MAX_HOLD_SECONDS = "3600"
   ```

   Replace the endpoint and model name with the values shown by your LM Studio server. The defaults are `http://192.168.1.146:1234/v1`, `qwen/qwen3.5-9b`, and `paper`.

   Settings can also be placed in the ignored local `.env` file; `.env.example` lists the available options. Startup validates endpoints, mode selectors, spread threshold, confirmation/hysteresis, data-quality, paper sizing, and notification/dashboard/log intervals. Invalid values stop startup with an error. The dashboard reports live prices and sampled per-pair spread statistics at the configured interval, including data ages and freshness status. The original `arbitrage_log.csv` remains an opportunity/LLM log; sampled numeric market observations are written separately to `spread_observations.csv`.

5. **Optionally configure private ntfy notifications** (PowerShell). This creates a random topic in the ignored local `.env` file; do not publish that file or share the topic:
   ```powershell
   $topic = [guid]::NewGuid().ToString("N")
   "NTFY_TOPIC_URL=https://ntfy.sh/$topic" | Set-Content -Encoding ascii .env
   ```

   If `NTFY_TOPIC_URL` is not set, notifications stay disabled. The public topic previously shown in repository history should be treated as exposed; use only a newly generated topic.

6. **Run the monitor**:
   ```powershell
   python main.py
   ```

   Press `Ctrl+C` to stop it. The CSV logs are created in the current working directory. No exchange API keys are needed for the public ticker requests.

7. **Analyze observations offline** (does not connect to the network):
   ```powershell
   python analyze_spreads.py
   python analyze_spreads.py .\spread_observations.csv --output .\spread_summary.csv
   ```

   The report includes per-pair mean, median, 90th/95th percentile, maximum, threshold-hit counts, positive estimated-net counts, and triggered opportunities. These historical statistics are diagnostic only and do not establish executable profitability.

   `ALERT_CONFIRMATION_SECONDS` is the uninterrupted time the spread must remain profitable before triggering. After an alert, the spread must fall below the trigger level minus `ALERT_REARM_HYSTERESIS_PERCENT` before another alert can trigger. A signal is evaluated only while both exchange prices are fresh and their local receive times are within the maximum skew.

   Simulation settings: `PAPER_MODE=portfolio` sizes each virtual position as a percentage of available cash; `PAPER_MODE=fixed` uses `PAPER_FIXED_POSITION_USDT`, capped by available cash. The default portfolio starts with 1,000 USDT and allocates 10% of available cash per position. Virtual positions are closed when the spread returns to the configured target in the simulated trade direction or reaches the maximum holding time. Estimated round-trip fees are deducted at close. This simplified paper ledger uses last-trade prices and is not a realistic execution backtest. Restarting the bot resets the in-memory paper balance and open positions; closed-position history remains in `paper_trades.csv`.

8. **Open the local web dashboard** (in a second PowerShell window, from the repository directory):
   ```powershell
   python dashboard.py
   ```

   Open `http://127.0.0.1:8080`. The dashboard reads the same `spread_observations.csv` file as the bot, refreshes every five seconds, and displays up to the latest 20 observations. It binds only to localhost and does not expose the telemetry to other machines. The status reflects the age of the latest CSV observation; it is not a direct health check of the bot process. The default threshold is `MIN_SPREAD_THRESHOLD` or `0.05%`, and observations are marked stale after 30 seconds. You can override settings with `DASHBOARD_PORT`, `DASHBOARD_STALE_AFTER_SECONDS`, or command-line arguments, for example:
   ```powershell
   python dashboard.py --csv .\spread_observations.csv --port 8081 --threshold 0.05 --stale-after 30
   ```

## 🧪 Tests

Run the unit tests locally with Python 3.12:

```powershell
py -3.12 -m unittest discover -s tests -t . -v
```

GitHub Actions runs the same test suite on pushes and pull requests to `main`.
