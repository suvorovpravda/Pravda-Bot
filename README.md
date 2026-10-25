# 🤖 Pravda-Bot

A Python cryptocurrency market-monitoring bot for Kraken and Binance. It estimates cross-exchange spreads, applies basic price and fee guards, and can ask a locally hosted model through LM Studio to analyze qualifying spreads.

> **Important:** This project uses public last-trade ticker data only and never places real orders. The paper simulator tracks virtual positions by default using these prices; estimates are not executable profit estimates, fills, or financial advice. `TRADING_MODE` is a display label and does not change execution behavior.

---

## 🚀 Key Features

* **Real-Time Multi-Exchange Monitoring**: Subscribes to public ticker WebSockets for BTC/USDT, ETH/USDT, and SOL/USDT on Kraken and Binance. It monitors last-trade prices, not order-book bid/ask prices.
* **Resilient WebSocket Reconnection**: Retries interrupted exchange streams with exponential backoff, bounded to 30 seconds with jitter; the delay resets after a stable connection.
* **Rich Dashboard**: Refreshes a compact per-symbol market and sampled-spread summary at a configurable interval instead of printing every ticker update.
* **European Gas Dashboard**: Serves a modern local view of reported ENTSOG cross-border physical-flow observations and linked GDELT news mentions, with source timestamps and an ENTSOG network schematic. News is not verified incident intelligence. No synthetic TTF quote is shown: a public live price requires a properly licensed provider. The crypto bot observations remain available as a secondary panel.
* **Live Market Board**: Publishes the bot's existing Kraken/Binance ticker updates to a local snapshot for the dashboard's live price table and in-session chart. EUR foreign-exchange reference rates are sourced from the ECB via Frankfurter and are explicitly labelled daily, not streaming quotes.
* **Safety Guards**:
  * Rejects prices that are missing, non-finite, zero, or negative.
  * Resets pending alert confirmation when a price tick is invalid and logs only the exception type/HTTP status for ntfy request errors.
  * Estimates net spread by subtracting a fixed 0.36% fee estimate from the absolute gross spread.
  * Uses a configurable minimum gross spread threshold (default `0.05%`) and rejects invalid threshold settings at startup.
  * Rejects non-finite, non-positive, or anomalously large spreads (default maximum `5%`) before paper simulation, arbitrage alerts, or LLM analysis. Finite rejected anomalies are logged with the `ANOMALY` status and excluded from offline spread statistics.
  * Skips LLM analysis when the estimated net spread is zero or negative.
  * Requires a candidate spread to remain profitable for a configurable confirmation interval and uses hysteresis before rearming alerts.
  * Rejects exchange prices older than 4 seconds by default or excessively desynchronized before spread evaluation; age is based on the exchanges' event timestamps when available. Snapshot freshness is checked before side effects and again when queued thread work starts. A stale signal is not recorded as triggered, and an alert already notified remains latched if its LLM analysis is later skipped; hysteresis still requires the spread to fall below the rearm threshold. Configure the stale limit with `MARKET_DATA_STALE_SECONDS`.
* **Local LLM Analysis**: Sends qualifying symbol and price data to the configured `LM_STUDIO_API_BASE` chat-completions endpoint. Use a trusted local LM Studio endpoint if the data must remain on your network.
* **Price-Movement Alerts**: Detects ticker changes over WebSockets and sends them only while both market feeds remain fresh and synchronized, rechecking when a queued ntfy call starts. ntfy notifications have a default five-second per-pair cooldown and a ten-second global send interval. Ticks suppressed by the global limit are not queued.
* **Arbitrage Alerts and Logging**: Sends a separate alert and logs an LLM analysis only when the estimated net spread becomes positive. `spread_observations.csv` records sampled numeric spread data (default every 2 seconds per pair), including threshold and fee values, for later offline analysis with `analyze_spreads.py`.
* **Paper Simulator**: Tracks a virtual portfolio or fixed-size positions without sending exchange orders. It estimates spread-capture P&L after configured fees and records closed simulations in `paper_trades.csv`; this is not a fill-accurate backtest.
* **Centralized Configuration Validation**: Validates endpoints, required model settings, and numeric intervals at startup with actionable errors.
* **No Order Placement**: The script does not use exchange API keys and contains no order-placement calls.

The 0.36% fee estimate is fixed and approximate. It does not account for slippage, withdrawal/network costs, latency, or exchange-specific fee tiers. No network fee is currently included in the net spread. The safe price-based helper accepts `fee_rate` as a fraction (`0.0036` = `0.36%`) and returns a fractional net rate (`0.01` = `1%`); the bot explicitly converts it to percentage points for logs and thresholds. The opportunity detector calculates absolute gross spread as `abs(Kraken - Binance) / min(Kraken, Binance)`, while the paper simulator tracks a signed spread as `(Kraken - Binance) / Binance`. Since these metrics use different denominators and purposes, their percentages are not directly interchangeable. The paper simulator uses last prices to estimate spread capture, but does not model order books, fills, funding, borrowing, or exchange constraints. The gross spread safety cap is configurable with `MAX_ALLOWED_SPREAD_PERCENT` (default `5.0%`). The minimum trigger threshold is configurable, not automatically optimized; use the observations as historical diagnostics rather than as evidence of executable profitability.

Notifications are disabled when `NTFY_TOPIC_URL` is unset. To enable them, configure a private, randomly generated topic URL in your local `.env`; never commit or share that topic. ntfy.sh topics are unlisted, not access-controlled, so anyone who learns the topic name may be able to read or publish notifications. After HTTP 429, sends pause according to `Retry-After` or an exponential backoff starting at 60 seconds (up to 15 minutes). `PRICE_ALERT_COOLDOWN_SECONDS=0` disables the per-pair cooldown but does not remove the global limit or server backoff.

---

## 🛠️ Tech Stack

* **Language**: Python 3.10+
* **Exchange Data**: Kraken and Binance public WebSocket ticker streams
* **AI / LLM**: LM Studio-compatible chat-completions API
* **Environment Configuration, WebSockets, Notifications and Terminal Output**: `python-dotenv`, `requests`, `websockets`, `rich`

---

## AI tools: configuration and responsibilities

The repository has **one AI integration at runtime**: the configured model served by LM Studio. Copilot, Gemini, and ChatGPT are development/review assistants, not services called by the bot.

| Tool | Role in this project | Configuration and data flow |
| --- | --- | --- |
| **Qwen via LM Studio** | Optional local analysis of a crypto spread that has already passed the bot's numeric, fee, market-data, threshold, and confirmation checks. Its response is informational; it cannot place exchange orders or override those checks. | Load a Qwen model in LM Studio and enable its local API server. Set `LM_STUDIO_API_BASE` to that server's `/v1` endpoint and `LM_MODEL_NAME` to the model identifier shown by LM Studio. `main.py` sends qualifying symbol and price data to the chat-completions endpoint. Keep the endpoint on a trusted local machine/network if this data must remain private. The bot does not use a Qwen API key. |
| **GitHub Copilot in VS Code** | Coding assistance: implementation, explanations, and test-driven changes in the editor. | Used interactively during development; it is not a dependency, runtime service, or configured exchange connection in this repository. |
| **Gemini** | Optional independent/manual review of selected code or a sanitized diff. | No Gemini SDK, API key, model setting, or automated request is configured here. Share only the minimum necessary review material through the service you choose. |
| **ChatGPT** | Optional independent/manual review, test-plan suggestions, and a second opinion on code changes. | No ChatGPT SDK, API key, model setting, or automated request is configured here. Share only the minimum necessary review material through the service you choose. |

### Suggested review workflow

1. Use Copilot for a small, scoped implementation and ask it to add or update tests.
2. Run the local test suite and inspect the complete Git diff before accepting or publishing changes.
3. For an independent review, provide Gemini or ChatGPT a sanitized diff plus the relevant requirements. Ask for concrete findings with file/line references, impact, confidence, and a reproducible test; verify every finding locally rather than applying suggestions blindly.
4. Treat AI output as untrusted advice. None of these assistants replaces code review, test results, exchange documentation, or a security audit.

Never include `.env`, API keys, private ntfy topic URLs, credentials, or unredacted private logs in prompts or public issues. The repository ignores `.env` and local CSV logs, but check the staged diff before every commit. AI assistance does not automatically make Gemini, Qwen, or ChatGPT a GitHub contributor: GitHub attribution requires a real commit author/co-author identity linked to a GitHub account. Do not invent co-author emails or rewrite published history to manufacture attribution.

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
   $env:MAX_ALLOWED_SPREAD_PERCENT = "5.0"
   $env:SPREAD_LOG_INTERVAL_SECONDS = "2"
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
   python analyze_paper_results.py
   ```

   `analyze_spreads.py` reports per-pair mean, median, 90th/95th percentile, maximum, threshold-hit counts, positive estimated-net counts, and triggered opportunities. `analyze_paper_results.py` reports valid observations, triggered opportunities, anomalies, and other health statuses using the `Market_Data_Status` column. These historical statistics are diagnostic only and do not establish executable profitability.

   `ALERT_CONFIRMATION_SECONDS` is the uninterrupted time the spread must remain profitable before triggering. After an alert, the spread must fall below the trigger level minus `ALERT_REARM_HYSTERESIS_PERCENT` before another alert can trigger. A signal is evaluated only while both exchange prices are fresh and their local receive times are within the maximum skew.

   Simulation settings: `PAPER_MODE=portfolio` sizes each virtual position as a percentage of available cash; `PAPER_MODE=fixed` uses `PAPER_FIXED_POSITION_USDT`, capped by available cash. The default portfolio starts with 1,000 USDT and allocates 10% of available cash per position. Virtual positions are closed when the spread returns to the configured target in the simulated trade direction or reaches the maximum holding time. Estimated round-trip fees are deducted at close. This simplified paper ledger uses last-trade prices and is not a realistic execution backtest. Restarting the bot resets the in-memory paper balance and open positions; closed-position history remains in `paper_trades.csv`.

8. **Launch the bot and live dashboard with one click** (Windows):
   - Double-click `Lancer_Pravda_Bot.bat` in the repository folder. It starts the market-data bot and the local dashboard in separate windows, then opens Chrome at `http://127.0.0.1:8080`.
   - Keep both windows open while using the dashboard. Closing the bot window stops the live exchange feeds; closing the dashboard window stops the web page server. The bot needs an internet connection for public exchange feeds. Local `.env` settings are loaded as usual.
   - To keep a one-click entry in Google Chrome, bookmark `http://127.0.0.1:8080` after launching it. The bookmark opens only while the bot and dashboard are running on this same PC; it is not a public/cloud-hosted website.

   The launcher uses `.venv\Scripts\python.exe` when available, otherwise it looks for Python 3.12 via the Windows `py` launcher. If dependencies are missing, install them once with `python -m pip install -r requirements.txt`.

   To start only the dashboard manually (in a second PowerShell window, from the repository directory):
   ```powershell
   python dashboard.py
   ```

   Open `http://127.0.0.1:8080`. The dashboard requests cross-border physical-flow records from the ENTSOG Transparency Platform (cached for five minutes), monitors relevant GDELT news mentions (cached for 15 minutes, with a BBC World RSS fallback), and refreshes the local crypto journal every five seconds. When the bot is running, its existing Kraken/Binance ticker streams publish prices to `market_snapshot.json`; the dashboard polls that local snapshot each second, rejects quotes older than ten seconds, and builds a chart from ticks observed while the page is open. The file is runtime-only and git-ignored. Foreign-exchange references are daily ECB rates through Frankfurter and display their publication date; they are not live interbank prices. Flow values retain ENTSOG's original units and source times; point observations are not deduplicated and must not be interpreted as national net imports or exports. The network view is schematic, not a geographic pipeline route map. News items link to their publishers and are not verified incidents or predictions. The TTF tile remains unavailable until a licensed data provider is configured; ICE/EEX access may require a subscription and public redistribution rights. The dashboard uses local CSS and JavaScript without a third-party CDN, binds only to localhost, and renders data as text. It does not expose telemetry to other machines by default. The default threshold is `MIN_SPREAD_THRESHOLD` or `0.05%`, and observations are marked stale after 30 seconds. You can override settings with `DASHBOARD_PORT`, `DASHBOARD_STALE_AFTER_SECONDS`, or command-line arguments, for example:
   ```powershell
   python dashboard.py --csv .\spread_observations.csv --lng-csv .\lng_observations.csv --port 8081 --threshold 0.05 --stale-after 30
   ```

9. **Write sample LNG benchmark observations** (standalone; no live quotes or shipping data are fetched):
   ```powershell
   python lng_observations.py
   python lng_observations.py --csv .\data\lng_samples.csv
   ```

   This appends sample TTF, JKM, and Henry Hub values to `lng_observations.csv` with status `SIMULATED`. These hard-coded examples are not current market prices, investment data, or vessel-tracking observations. The default local CSV is ignored by Git.

## 🧪 Tests

Run the unit tests locally with Python 3.12:

```powershell
py -3.12 -m unittest discover -s tests -t . -v
```

GitHub Actions runs the same test suite on pushes and pull requests to `main`.
