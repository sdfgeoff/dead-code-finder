from typing import Protocol

from api.framework import Depends, Router


class Service(Protocol):
    async def run(self) -> str: ...


class ExampleEntity:
    def __init__(self, user_id: int) -> None:
        self.user_id = user_id


class RealService:
    async def run(self) -> str:
        return "real"


def get_service() -> Service:
    return RealService()


def get_user() -> ExampleEntity:
    return ExampleEntity(1)


class ServiceConnection:
    def __init__(self, service: Service) -> None:
        self.service = service

    async def run(self) -> str:
        return await self.service.run()


router = Router()


@router.get("/items")
async def list_items(
    entity: ExampleEntity = Depends(get_user),
    service: Service = Depends(get_service),
) -> str:
    _ = entity.user_id
    connection = ServiceConnection(service)
    return await connection.run()
