from typing import Annotated, Generic, Literal, TypeVar, Union

from fastapi import APIRouter
from pydantic import BaseModel, Field
from typing_extensions import TypeAliasType


router = APIRouter()

T = TypeVar("T")


class RequestModel(BaseModel):
    name: str


class FirstVariant(BaseModel):
    kind: Literal["first"]
    value: int


class SecondVariant(BaseModel):
    kind: Literal["second"]
    label: str


BoundaryPayload = TypeAliasType(
    "BoundaryPayload",
    Annotated[
        Union[FirstVariant, SecondVariant],
        Field(discriminator="kind"),
    ],
)


class ResponseModel(BaseModel, Generic[T]):
    item: T
    total: int


class NotExposed(BaseModel):
    field: str


@router.post("/items")
def create_item(payload: RequestModel) -> ResponseModel[BoundaryPayload]:
    raise NotImplementedError
