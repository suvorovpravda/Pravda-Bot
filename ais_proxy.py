from collections import namedtuple
from datetime import datetime, timezone
import os

AISMessage = namedtuple('AISMessage', ['mmsi', 'lat', 'lon', 'sog', 'cog', 'timestamp', 'source'])

def get_ais_recent(limit: int = 100):
    provider = os.getenv('AIS_PROVIDER', 'off')
    if provider != 'aisstream.io':
        return []
    # Sample data for demonstration
    now = datetime.now(timezone.utc).isoformat(timespec='seconds').replace('+00:00', 'Z')
    samples = [
        AISMessage(123456789, 43.5, 7.2, 12.5, 90, now, 'aisstream.io'),
        AISMessage(987654321, 42.9, 3.8, 8.3, 180, now, 'aisstream.io'),
        AISMessage(555123456, 44.2, 5.5, 5.0, 45, now, 'aisstream.io'),
    ]
    return samples[:limit]
