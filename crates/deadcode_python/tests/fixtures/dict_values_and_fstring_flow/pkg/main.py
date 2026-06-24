class Registered:
    name: str
    max_calls: int | None
    unused: str


class Registry:
    def __init__(self):
        self.tools: dict[str, Registered] = {}

    def schemas(self) -> list[str]:
        schemas: list[str] = []
        for tool in self.tools.values():
            description = tool.name
            if tool.max_calls is not None:
                suffix = f"Limit: {tool.max_calls}"
                description = description + suffix if description else suffix.strip()
            schemas.append(description)
        return schemas


Registry().schemas()
