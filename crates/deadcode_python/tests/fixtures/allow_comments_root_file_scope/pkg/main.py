# dead-code-finder: allow-file


def helper() -> str:
    return "kept"


def api_surface() -> str:
    return helper()


class Client:
    value: str

    def endpoint(self) -> str:
        return helper()
