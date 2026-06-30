from typedlib import parse


class Payload:
    used: str
    unused: str


def main() -> str:
    payload = parse(Payload, {"used": "value"})
    return payload.used


main()
