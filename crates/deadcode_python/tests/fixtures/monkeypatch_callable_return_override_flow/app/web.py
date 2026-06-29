from app.redis_client import get_client


def write_value() -> None:
    get_client().set("key", "value")


def read_value() -> str | None:
    return get_client().get("key")
