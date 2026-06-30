from typing import Protocol


class Worker:
    def __call__(self, value: int) -> int:
        return value + 1

    def unused(self) -> int:
        return 0


class UnusedWorker:
    def __call__(self, value: int) -> int:
        return value


class Endpoint(Protocol):
    def __call__(self, value: int) -> int:
        raise NotImplementedError


class Poller:
    def __init__(self, endpoint: Endpoint):
        self.endpoint: Endpoint = endpoint

    def poll(self) -> int:
        return self.endpoint(1)


def run() -> int:
    worker = Worker()
    return worker(1)


def poll(endpoint: Endpoint) -> int:
    return Poller(endpoint).poll()


run()
poll(Worker())
