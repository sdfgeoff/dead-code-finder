from typing import Any, cast


class Result:
    error: str | None
    unused: str


def parse(response_obj: object) -> Result:
    response_payload = (
        cast(dict[str, Any], response_obj)
        if isinstance(response_obj, dict)
        else cast(dict[str, Any], {})
    )
    return Result(error=cast(str | None, response_payload.get("error")))


def run() -> str | None:
    return parse({"error": "channel_not_found"}).error


run()
