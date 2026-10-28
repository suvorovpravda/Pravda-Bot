from collections import namedtuple

AISMessage = namedtuple('AISMessage', ['mmsi', 'lat', 'lon', 'sog', 'cog', 'timestamp', 'source'])

def get_ais_recent(limit: int = 100):
    """No receiver is connected to this store; never invent vessel positions.

    Setting a provider name alone does not supply observations. Dashboard routes
    return HTTP 503 until a real, validated receiver is integrated here.
    """
    return []
