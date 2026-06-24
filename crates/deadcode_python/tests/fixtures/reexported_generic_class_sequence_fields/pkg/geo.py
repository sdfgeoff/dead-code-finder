from typing import Generic, TypeVar

PropertyType = TypeVar("PropertyType")


class Feature(Generic[PropertyType]):
    properties: PropertyType
    unused: int
