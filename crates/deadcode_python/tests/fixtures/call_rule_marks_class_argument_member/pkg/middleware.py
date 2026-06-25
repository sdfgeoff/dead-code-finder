class LoggedMiddleware:
    async def dispatch(self) -> None:
        self.record_request()

    def record_request(self) -> None:
        pass


class UnusedMiddleware:
    async def dispatch(self) -> None:
        pass
