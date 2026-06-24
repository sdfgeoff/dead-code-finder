from typing import Any, Generic, TypeVar


Arg = TypeVar("Arg")
Res = TypeVar("Res")


class Tool(Generic[Arg, Res]):
    name: str

    async def execute(self, arg: Arg) -> Res:
        return "base"


class LiveTool(Tool[int, str]):
    name = "live"

    async def execute(self, arg: int) -> str:
        return "live"


class DeadTool(Tool[int, str]):
    name = "dead"

    async def execute(self, arg: int) -> str:
        return "dead"


class RegisteredTool:
    instance: Tool[Any, Any]


class Registry:
    def __init__(self, tools: list[Tool[Any, Any]] | None = None) -> None:
        self.registered = {
            type(tool).name: RegisteredTool(instance=tool) for tool in (tools or [])
        }

    async def call_first(self) -> str:
        registered = self.registered.get("live")
        if registered is None:
            return ""
        return await registered.instance.execute(1)


async def run() -> str:
    registry = Registry([LiveTool()])
    return await registry.call_first()


run()
