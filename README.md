# 🤖 Pravda-Bot

A Python cryptocurrency market-monitoring bot for Kraken and Binance. It estimates cross-exchange spreads, applies basic price and fee guards, and can ask a locally hosted model through LM Studio to analyze qualifying spreads.

> **Important:** This project monitors public ticker data only. It does not place, simulate, or manage trades. `TRADING_MODE` is displayed as a label; it does not change execution behavior. Spread estimates are not executable profit estimates or financial advice.

---

## 🚀 Key Features

* **Multi-Exchange Monitoring**: Checks the `last` ticker price for BTC/USDT, ETH/USDT, and SOL/USDT on Kraken and Binance every 10 seconds. It does not currently read order-book bid/ask prices.
* **Safety Guards**:
  * Rejects prices that are missing, non-finite, zero, or negative.
  * Estimates net spread by subtracting a fixed 0.36% fee estimate from the absolute gross spread.
  * Skips LLM analysis when the estimated net spread is zero or negative.
* **Local LLM Analysis**: Sends qualifying symbol and price data to the configured `LM_STUDIO_API_BASE` chat-completions endpoint. Use a trusted local LM Studio endpoint if the data must remain on your network.
* **Alerts and Logging**: Prints terminal alerts, sends the symbol and net spread to the configured ntfy topic, and appends qualifying analyses to `arbitrage_log.csv`.
* **No Order Placement**: The script does not use exchange API keys and contains no order-placement calls.

The 0.36% fee estimate is fixed and approximate. It does not account for slippage, withdrawal/network costs, latency, or exchange-specific fee tiers. The gross spread uses an absolute price difference, so it does not determine which exchange is cheaper to buy from and which is more expensive to sell on.

The ntfy topic is hosted by `ntfy.sh` by default, so notification content is sent to that external service. Set `NTFY_TOPIC_URL` to a different topic URL if preferred.

---

## 🛠️ Tech Stack

* **Language**: Python 3.10+
* **Exchange Data**: CCXT public ticker endpoints
* **AI / LLM**: LM Studio-compatible chat-completions API
* **Environment Configuration and Notifications**: `python-dotenv`, `requests`

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
   $env:LM_MODEL_NAME = "qwen-local"
   $env:TRADING_MODE = "paper"
   $env:NTFY_TOPIC_URL = "https://ntfy.sh/pravda_bot_vincent_9832"
   ```

   Replace the endpoint and model name with the values shown by your LM Studio server. The defaults are `http://192.168.1.146:1234/v1`, `qwen-local`, and `paper`.

5. **Run the monitor**:
   ```powershell
   python main.py
   ```

   Press `Ctrl+C` to stop it. The CSV log is created in the current working directory. No exchange API keys are needed for the public ticker requests.
