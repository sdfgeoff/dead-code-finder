import json
import requests


def get_keys(keys_url: str) -> object:
    if keys_url.startswith("http"):
        response = requests.get(keys_url)
        payload = response.json()
    else:
        payload = json.loads("{}")
    return payload.get("keys")


get_keys("https://example.com")
