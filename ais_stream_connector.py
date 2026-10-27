"""
AIS Stream Connector for Pravda-Bot
Asynchronous integration with aisstream.io free non-commercial WebSocket feed.
Follows user-provided script pattern with robust reconnection and bounding box filtering.
"""

import asyncio
import json
import os
from typing import List, Dict

import websockets

API_KEY = os.getenv("AISSTREAM_API_KEY", "VOTRE_CLE_API_AISSTREAM")
# Default bounding box: Western Mediterranean / French coast
DEFAULT_BOUNDING_BOXES = [
    [[41.0, 3.0], [44.0, 9.0]]
]

URI = "wss://stream.aisstream.io/v0/stream"


class AISStreamConnector:
    def __init__(self, api_key: str = API_KEY, bounding_boxes: List[List[List[float]]] = None):
        self.api_key = api_key
        self.bounding_boxes = bounding_boxes or DEFAULT_BOUNDING_BOXES
        self.subscribers: List[Callable[[Dict], None]] = []

    def subscribe(self, callback):
        self.subscribers.append(callback)

    async def _handle_message(self, message_json: str):
        try:
            message = json.loads(message_json)
        except json.JSONDecodeError:
            return
        if message.get("MessageType") != "PositionReport":
            return
        meta = message.get("MetaData", {})
        pos = message.get("Message", {}).get("PositionReport", {})
        ship_name = meta.get("ShipName", "INCONNU")
        mmsi = meta.get("MMSI")
        lat = pos.get("Latitude")
        lon = pos.get("Longitude")
        speed = pos.get("Sog")
        course = pos.get("Cog")
        payload = {
            "ship_name": ship_name,
            "mmsi": mmsi,
            "lat": lat,
            "lon": lon,
            "speed_kn": speed,
            "course": course,
            "source": "aisstream.io",
            "raw": message,
        }
        for cb in self.subscribers:
            try:
                cb(payload)
            except Exception:
                pass

    async def connect_loop(self):
        subscription_message = {
            "APIKey": self.api_key,
            "BoundingBoxes": self.bounding_boxes,
        }
        while True:
            try:
                print("[*] Connexion au flux AIS en cours...")
                async with websockets.connect(URI) as ws:
                    print("[*] Connecté. Envoi de la souscription...")
                    await ws.send(json.dumps(subscription_message))
                    async for msg in ws:
                        await self._handle_message(msg)
            except websockets.exceptions.ConnectionClosed as e:
                print(f"[-] Connexion perdue ({e}). Reconnexion dans 5 secondes...")
                await asyncio.sleep(5)
            except Exception as e:
                print(f"[-] Erreur critique dans le flux AIS : {e}")
                await asyncio.sleep(5)


async def run_ais_connector():
    connector = AISStreamConnector()
    # Example subscriber: print
    def printer(data):
        print(f"[AIS] {data['ship_name']} (MMSI: {data['mmsi']}) -> Lat: {data['lat']}, Lon: {data['lon']} | Vitesse: {data['speed_kn']} kn")
    connector.subscribe(printer)
    await connector.connect_loop()


if __name__ == "__main__":
    asyncio.run(run_ais_connector())
