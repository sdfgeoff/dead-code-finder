from typing import Generic, Sequence, TypeVar


Id = TypeVar("Id")
Geometry = TypeVar("Geometry")
Properties = TypeVar("Properties")


class Feature(Generic[Id, Geometry, Properties]):
    geometry: Geometry
    properties: Properties


class FeatureCollection(Generic[Id, Geometry, Properties]):
    features: Sequence[Feature[Id, Geometry, Properties]]


class BoundaryProperties:
    title_area: float
    unused: str
