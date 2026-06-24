from typing import Generic, TypeVar


GeometryType = TypeVar("GeometryType")
PropertyType = TypeVar("PropertyType")


class Properties:
    name: str
    unused: str


class Feature(Generic[GeometryType, PropertyType]):
    geometry: GeometryType
    properties: PropertyType


class FeatureCollection(Generic[GeometryType, PropertyType]):
    features: list[Feature[GeometryType, PropertyType]]


class Content(Generic[GeometryType, PropertyType]):
    example_items: dict[str, FeatureCollection[GeometryType, PropertyType]]


class Response(Generic[GeometryType, PropertyType]):
    vector_query: Content[GeometryType, PropertyType]
