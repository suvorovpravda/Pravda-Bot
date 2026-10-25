# 🤖 Pravda-Bot

An agentic cryptocurrency arbitrage bot built in Python, leveraging multi-exchange data (Kraken & Binance), strict risk management guards, and local LLM intelligence (via LM Studio / RTX 3090) for real-time opportunity analysis.

---

## 🚀 Key Features

* **Multi-Exchange Surveillance**: Continuously monitors real-time order book prices (`Ask`/`Bid`) across major pairs (`BTC/USDT`, `ETH/USDT`, `SOL/USDT`).
* **Advanced Safety Guards**: 
  * Strict price validation (rejects zero, negative, or missing values).
  * Net spread calculation factoring in an estimated round-trip fee threshold (0.36%).
  * Automatic filtering of non-profitable spreads ($\le 0\%$ net).
* **Local LLM Integration**: Automatically queries a locally hosted Qwen model via LM Studio to qualitatively analyze valid arbitrage spreads without exposing data externally.
* **Alerting & Logging**: Triggers audible terminal alerts (`\a`) and color-coded visual outputs upon detecting viable opportunities, while appending structured logs to a local CSV file.
* **Paper Trading Mode**: Operates completely risk-free in simulated mode by default.

---

## 🛠️ Tech Stack

* **Language**: Python 3.10+
* **API Wrapper**: CCXT
* **AI / LLM**: LM Studio (Local Inference)
* **Environment Control**: `python-dotenv`, `requests`

---

## ⚙️ Installation & Quickstart

1. **Clone the repository**:
   ```bash
   git clone [https://github.com/vincentdsb/Pravda-Bot.git](https://github.com/vincentdsb/Pravda-Bot.git)
   cd Pravda-Bot
