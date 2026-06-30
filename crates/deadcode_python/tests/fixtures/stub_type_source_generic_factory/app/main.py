from typedlib import parse


class Payload:
    used: str
    unused: str


def main() -> str:
    payload = parse(model=Payload, data={"used": "value"})
    return payload.used


main()
