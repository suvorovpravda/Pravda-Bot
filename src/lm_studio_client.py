"""LM Studio Client - Interrogation du modèle local sur RTX 3090."""

from __future__ import annotations

import logging
import os
from typing import Any

logger = logging.getLogger("Pravda_LMStudio")


class LMStudioClient:
    """Client pour interroger le serveur local de LM Studio.

    Utilise l'API compatible OpenAI de LM Studio :
    - base_url: http://localhost:1234/v1 (ou IP locale configurée)
    - Modèle par défaut: qwen3.5-9b (gratuit, sans quota)
    """

    def __init__(self):
        """Initialiser le client avec configuration LM Studio."""
        self._base_url = os.getenv(
            "LM_STUDIO_API_BASE", 
            "http://localhost:1234/v1"
        )
        self._model_name = os.getenv("LM_MODEL_NAME", "qwen3.5-9b")

    def query_local_model(self, prompt: str, system_prompt: str = "") -> str:
        """Envoyer un prompt au modèle local et récupérer la réponse.

        Args:
            prompt: Message utilisateur à analyser
            system_prompt: Instructions système optionnelles pour le modèle

        Returns:
            Chaîne de texte avec la réponse du modèle

        Raises:
            ConnectionError: Si le serveur LM Studio n'est pas lancé
            TimeoutError: Si la requête dépasse le timeout
        """
        try:
            import requests
            
            # Construire les messages pour le chat complet
            messages = [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt}
            ]

            # Envoyer la requête au serveur LM Studio
            response = requests.post(
                f"{self._base_url}/chat/completions",
                json={
                    "model": self._model_name,
                    "messages": messages,
                    "temperature": 0.2,  # Faible température pour analyse factuelle
                    "max_tokens": 500,
                },
                timeout=30  # Timeout de 30 secondes
            )

            if response.status_code == 200:
                result = response.json()
                return result.get('choices', [{}])[0].get('message', {}).get('content', '')
            
            elif response.status_code == 404:
                raise ConnectionError(
                    f"Serveur LM Studio non trouvé à {self._base_url}. "
                    "Vérifiez que le serveur est lancé et accessible."
                )
            
            else:
                error_msg = response.text or "Erreur inconnue du serveur"
                logger.error("Erreur HTTP %s de LM Studio: %s", response.status_code, error_msg)
                return f"Erreur LLM (HTTP {response.status_code}): {error_msg}"

        except requests.exceptions.ConnectionError as connection_error:
            raise ConnectionError(
                f"Impossible de se connecter à LM Studio à {self._base_url}. "
                "Vérifiez que le serveur est lancé et accessible sur le réseau."
            ) from connection_error
        
        except requests.exceptions.Timeout:
            raise TimeoutError(
                "Requête au modèle local dépassée (timeout 30s). "
                "Le modèle peut être trop chargé ou la requête trop complexe."
            )
        
        except Exception as error:
            logger.error("Erreur lors de l'appel à LM Studio: %s", error)
            return f"Erreur LLM: {error}"
