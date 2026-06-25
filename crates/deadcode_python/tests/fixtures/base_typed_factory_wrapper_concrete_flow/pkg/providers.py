from typing import Callable


class Connection:
    def lookup(self, key: str) -> bytes | None:
        raise NotImplementedError

    def write(self, key: str, value: bytes | None) -> None:
        raise NotImplementedError


class NetworkConnection(Connection):
    def lookup(self, key: str) -> bytes | None:
        return b"network"

    def write(self, key: str, value: bytes | None) -> None:
        pass


class MemoryConnection(Connection):
    def lookup(self, key: str) -> bytes | None:
        return b"memory"

    def write(self, key: str, value: bytes | None) -> None:
        pass


class WrapperConnection(Connection):
    def __init__(self, providers: list[Connection]):
        self.providers = providers

    def _try_with_provider(self, operation: Callable[[Connection], bytes | None]):
        for provider in self.providers:
            value = operation(provider)
            if value is not None:
                return value
        return None

    def lookup(self, key: str) -> bytes | None:
        return self._try_with_provider(lambda provider: provider.lookup(key))

    def write(self, key: str, value: bytes | None) -> None:
        for provider in self.providers:
            provider.write(key, value)


def create_connection() -> Connection:
    providers = [NetworkConnection(), MemoryConnection()]
    return WrapperConnection(providers)
