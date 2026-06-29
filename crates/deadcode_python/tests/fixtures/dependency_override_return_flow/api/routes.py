from typing import Protocol

from api.framework import Depends, Router


class Service(Protocol):
    def run(self) -> str: ...


class RealService:
    def run(self) -> str:
        return "real"


def get_service() -> Service:
    return RealService()


class ServiceConnection:
    def __init__(self, service: Service) -> None:
        self.service = service

    def run(self) -> str:
        return self.service.run()


router = Router()


@router.get("/items")
def list_items(service: Service = Depends(get_service)) -> str:
    connection = ServiceConnection(service)
    return connection.run()
