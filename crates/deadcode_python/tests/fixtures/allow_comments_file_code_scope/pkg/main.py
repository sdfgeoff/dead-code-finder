# dead-code-finder: allow-file DCF001


def helper() -> str:
    return "kept"


def api_surface() -> str:
    return helper()


class DeadClient:
    value: str

    def endpoint(self) -> str:
        return helper()
