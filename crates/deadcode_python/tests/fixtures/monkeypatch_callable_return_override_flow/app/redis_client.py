from typing import Protocol


class RedisClient(Protocol):
    def set(self, key: str, value: str) -> None: ...
    def get(self, key: str) -> str | None: ...
    def publish(self, channel: str, message: str) -> None: ...


class RealRedisClient:
    def set(self, key: str, value: str) -> None:
        pass

    def get(self, key: str) -> str | None:
        return None

    def publish(self, channel: str, message: str) -> None:
        pass


def get_client() -> RedisClient:
    return RealRedisClient()
