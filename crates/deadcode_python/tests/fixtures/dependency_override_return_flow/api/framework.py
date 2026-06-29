class App:
    def __init__(self) -> None:
        self.dependency_overrides = {}

    def include_router(self, router: object) -> None:
        pass


class Router:
    def get(self, path: str):
        def decorate(func):
            return func

        return decorate


def Depends(func):
    return func
