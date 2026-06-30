from abc import ABC


class CacheConnection(ABC):
    def array_key_lookup(self, keys: list[str]) -> list[bytes | None]:
        raise NotImplementedError

    def insert_items(self, items: list[bytes]) -> None:
        raise NotImplementedError

    def get_item(self, key: str) -> bytes | None:
        raise NotImplementedError

    def set_item(self, key: str, value: bytes | None) -> None:
        raise NotImplementedError


class MemoryCacheConnection(CacheConnection):
    def array_key_lookup(self, keys: list[str]) -> list[bytes | None]:
        return [self.get_item(key) for key in keys]

    def insert_items(self, items: list[bytes]) -> None:
        for item in items:
            self.set_item("key", item)

    def get_item(self, key: str) -> bytes | None:
        return None

    def set_item(self, key: str, value: bytes | None) -> None:
        pass


class TrackedCacheConnection(CacheConnection):
    def __init__(self, cache: CacheConnection):
        self.cache = cache
        self.lookups: list[list[str]] = []

    def array_key_lookup(self, keys: list[str]) -> list[bytes | None]:
        self.lookups.append(keys)
        return self.cache.array_key_lookup(keys)

    def insert_items(self, items: list[bytes]) -> None:
        return self.cache.insert_items(items)

    def get_item(self, key: str) -> bytes | None:
        return self.cache.get_item(key)

    def set_item(self, key: str, value: bytes | None) -> None:
        return self.cache.set_item(key, value)


class UnusedCacheConnection(CacheConnection):
    def array_key_lookup(self, keys: list[str]) -> list[bytes | None]:
        return []

    def insert_items(self, items: list[bytes]) -> None:
        pass

    def get_item(self, key: str) -> bytes | None:
        return None

    def set_item(self, key: str, value: bytes | None) -> None:
        pass


def main() -> None:
    cache = TrackedCacheConnection(MemoryCacheConnection())
    cache.array_key_lookup(["key"])
    cache.insert_items([b"value"])


main()
