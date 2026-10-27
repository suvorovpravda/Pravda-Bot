"""
Terminal enhancements for Pravda-Bot
- Historical price charts multi-scale
- FX info with source/age
- Source status single view
- Daily summary export
"""
from datetime import datetime, timezone
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.columns import Columns
from rich.text import Text
from rich import box
import csv
import io
from pathlib import Path

from market_history import get_market_store
from live_market_data import get_reference_fx_rates
from energy_data import get_entsog_flows, get_geopolitical_news, get_ttf_price_status

console = Console()


def format_fx_info():
    try:
        fx = get_reference_fx_rates()
    except Exception as e:
        return f"FX indisponible: {e}"
    base = fx.get("base","EUR")
    date = fx.get("date","?")
    age_days = fx.get("source_age_days", "?")
    freshness = fx.get("freshness","?")
    rates = fx.get("rates",{})
    usd_rate = rates.get("USD", "?")
    chf_rate = rates.get("CHF", "?")
    # USD per EUR, so EUR/USD = 1/USD
    # We show EUR->USD and EUR->CHF
    txt = (
        f"Source: {fx.get('source','Frankfurter')} | Date: {date} | Âge: {age_days}j | Fraîcheur: {freshness}\n"
        f"EUR→USD: {usd_rate} | EUR→CHF: {chf_rate} | Base: {base}"
    )
    return txt


def source_status_view():
    table = Table(title="État des sources", box=box.ROUNDED)
    table.add_column("Source", style="cyan")
    table.add_column("Statut", style="magenta")
    table.add_column("Dernier événement", style="green")
    table.add_column("Détails", style="white")
    # Market
    table.add_row("Kraken", "live", "voir dashboard", "WebSocket")
    table.add_row("Binance", "live", "voir dashboard", "WebSocket")
    # ENTSOG
    try:
        flows = get_entsog_flows()
        table.add_row("ENTSOG", "live", flows.get("retrieved_at","-"), f"{len(flows.get('observations',[]))} obs")
    except Exception as e:
        table.add_row("ENTSOG", "indisponible", "-", str(e))
    # GDELT
    try:
        news = get_geopolitical_news()
        table.add_row("GDELT/BBC", "live", news.get("retrieved_at","-"), f"{len(news.get('items',[]))} articles")
    except Exception as e:
        table.add_row("GDELT/BBC", "indisponible", "-", str(e))
    # FX
    try:
        fx = get_reference_fx_rates()
        table.add_row("Frankfurter BCE", fx.get("freshness","?"), fx.get("date","?"), f"Âge {fx.get('source_age_days',0)}j")
    except Exception as e:
        table.add_row("Frankfurter BCE", "indisponible", "-", str(e))
    return table


def historical_chart(symbol, exchange, period="day"):
    store = get_market_store()
    try:
        data = store.history(symbol, exchange, period)
    except Exception as e:
        return Panel(f"Erreur historique {symbol}: {e}", title="Historique")
    if not data:
        return Panel("Aucune donnée", title=f"{symbol}/{exchange} {period}")
    # Simple ASCII sparkline using rich
    prices = [d["price"] for d in data]
    min_p = min(prices)
    max_p = max(prices)
    # normalize to 10 chars
    def bar(p):
        if max_p == min_p:
            return "█"
        norm = (p - min_p) / (max_p - min_p)
        blocks = " ▁▂▃▄▅▆▇█"
        return blocks[int(norm * 7) + 1]
    spark = "".join(bar(p) for p in prices)
    table = Table(box=box.SIMPLE)
    table.add_column("Heure")
    table.add_column("Prix")
    table.add_column("Low")
    table.add_column("High")
    table.add_column("Ticks")
    for d in data[-20:]:
        table.add_row(d["timestamp"], f"{d['price']:.2f}", f"{d['low']:.2f}", f"{d['high']:.2f}", str(d["ticks"]))
    panel = Panel(Columns([table, Panel(spark, title="Sparkline")]), title=f"{symbol}/{exchange} {period}")
    return panel


def export_daily_summary_csv(path="daily_summary.csv"):
    store = get_market_store()
    day = datetime.now(timezone.utc).date()
    # Market summary
    output = io.StringIO(newline="")
    writer = csv.writer(output)
    writer.writerow(["Type","Source","Date","Métrique","Valeur","Unité","Détails","URL"])
    # Market
    for s in store.daily_market_summary(day):
        writer.writerow(["market", s["exchange"], day.isoformat(), s["symbol"], f"{s['close']}", "USDT", "daily", ""])
    # Energy
    try:
        flows = get_entsog_flows()
        for obs in flows.get("observations", [])[:20]:
            writer.writerow(["energy", "ENTSOG", day.isoformat(), obs.get("direction",""), obs.get("value",""), obs.get("unit",""), obs.get("point",""), flows.get("source_url","")])
    except Exception:
        pass
    # News
    try:
        news = get_geopolitical_news()
        for item in news.get("items", [])[:20]:
            writer.writerow(["news", news.get("source","GDELT"), day.isoformat(), item.get("title",""), "", "", item.get("published",""), item.get("url","")])
    except Exception:
        pass
    Path(path).write_text(output.getvalue(), encoding="utf-8")
    return path


def print_enhanced_dashboard():
    fx_panel = Panel(format_fx_info(), title="FX référence BCE", box=box.ROUNDED)
    sources_panel = Panel(source_status_view(), title="Sources")
    # Historical examples
    charts = []
    store = get_market_store()
    for sym in ["BTC/USDT","ETH/USDT"]:
        charts.append(historical_chart(sym, "Binance", "day"))
    console.print(Panel(Columns([fx_panel, sources_panel]), title="Vue d'ensemble"))
    for c in charts:
        console.print(c)
