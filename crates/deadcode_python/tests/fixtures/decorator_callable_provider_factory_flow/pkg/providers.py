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
