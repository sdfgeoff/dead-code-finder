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

    def lookup(self, key: str) -> bytes | None:
        for provider in self.providers:
            value = provider.lookup(key)
            if value is not None:
                return value
        return None

    def write(self, key: str, value: bytes | None) -> None:
        for provider in self.providers:
            provider.write(key, value)


def create_connection() -> Connection:
    providers = [NetworkConnection(), MemoryConnection()]
    return WrapperConnection(providers)
