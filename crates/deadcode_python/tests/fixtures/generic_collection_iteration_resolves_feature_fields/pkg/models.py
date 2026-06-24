from typing import Generic, Sequence, TypeVar


IdType = TypeVar("IdType")
GeometryType = TypeVar("GeometryType")
PropertyType = TypeVar("PropertyType")


class Geometry:
    def dump(self):
        pass


class Properties:
    def label(self):
        pass


class Feature(Generic[IdType, GeometryType, PropertyType]):
    id: IdType
    geometry: GeometryType
    properties: PropertyType
    unused: PropertyType


class FeatureCollection(Generic[IdType, GeometryType, PropertyType]):
    features: Sequence[Feature[IdType, GeometryType, PropertyType]]
