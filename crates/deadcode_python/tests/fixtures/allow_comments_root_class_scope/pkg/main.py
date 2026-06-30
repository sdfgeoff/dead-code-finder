def helper() -> str:
    return "kept"


# dead-code-finder: allow
class Client:
    value: str

    def endpoint(self) -> str:
        return helper()

    def other_endpoint(self) -> str:
        return self.value


class DeadClient:
    value: str

    def endpoint(self) -> str:
        return helper()
