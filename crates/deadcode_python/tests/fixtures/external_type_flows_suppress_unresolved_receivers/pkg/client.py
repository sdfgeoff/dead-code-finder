import httpx


class Client:
    def __init__(self):
        self.client = httpx.Client()

    def run(self):
        response = self.client.post("https://example.com")
        response.raise_for_status()
        return response.json()
