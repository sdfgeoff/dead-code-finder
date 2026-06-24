from typing import Generic, Sequence, TypeVar


IdType = TypeVar("IdType")
GeometryType = TypeVar("GeometryType")
PropertyType = TypeVar("PropertyType")
MetadataType = TypeVar("MetadataType")


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


class BaseModel:
    pass


class FeatureCollection(
    BaseModel, Generic[IdType, GeometryType, PropertyType, MetadataType]
):
    features: Sequence[Feature[IdType, GeometryType, PropertyType]]
    metadata: MetadataType
