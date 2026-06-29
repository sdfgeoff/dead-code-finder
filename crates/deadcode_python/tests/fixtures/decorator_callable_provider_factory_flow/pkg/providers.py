class Connection:
    def lookup(self, keys: list[str]) -> list[bytes | None]:
        raise NotImplementedError

    def write(self, keys: list[str]) -> None:
        raise NotImplementedError


class RedisConnection(Connection):
    def lookup(self, keys: list[str]) -> list[bytes | None]:
        return [None for _ in keys]

    def write(self, keys: list[str]) -> None:
        pass


class MemoryConnection(Connection):
    def lookup(self, keys: list[str]) -> list[bytes | None]:
        return [None for _ in keys]

    def write(self, keys: list[str]) -> None:
        pass


class TrackedConnection(Connection):
    def __init__(self, connection: Connection) -> None:
        self.connection = connection

    def lookup(self, keys: list[str]) -> list[bytes | None]:
        return self.connection.lookup(keys)

    def write(self, keys: list[str]) -> None:
        self.connection.write(keys)

    def unused(self) -> None:
        pass
