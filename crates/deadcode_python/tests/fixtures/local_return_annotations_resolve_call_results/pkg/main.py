from pkg.client import Client


class Pipeline:
    def __init__(self, client: Client):
        self.client = client

    def run(self):
        result = self.client.create()
        return result.id, result.created


pipeline = Pipeline(Client())
pipeline.run()
