from collections.abc import Callable
from typing_extensions import TypedDict


class HandlerMap(TypedDict, total=False):
    encode: Callable[[str], bytes]
    decode: Callable[[bytes], str]
    unused: Callable[[str], str]


def make_handlers() -> HandlerMap:
    def encode(value: str) -> bytes:
        return value.encode("utf-8")

    def decode(value: bytes) -> str:
        return value.decode("utf-8")

    return {"encode": encode, "decode": decode}


if __name__ == "__main__":
    make_handlers()
