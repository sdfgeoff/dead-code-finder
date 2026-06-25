from typing import Generic, Literal, Type, TypeVar

from pydantic import BaseModel, TypeAdapter
from example_geojson import (
    FeatureGeometryType,
    FeatureIdType,
    FeaturePropertyType,
    GeometryTypes,
)


PayloadType = TypeVar("PayloadType", bound=BaseModel)


class Payload(BaseModel):
    name: str
    s3: str | None = None


class Feature(
    BaseModel, Generic[FeatureIdType, FeatureGeometryType, FeaturePropertyType]
):
    type: Literal["Feature"]
    id: FeatureIdType | None = None
    properties: FeaturePropertyType
    geometry: FeatureGeometryType | None = None


class FeatureCollection(
    BaseModel,
    Generic[FeatureIdType, FeatureGeometryType, FeaturePropertyType],
):
    type: Literal["FeatureCollection"]
    features: list[Feature[FeatureIdType, FeatureGeometryType, FeaturePropertyType]]


class VectorQueryContent(BaseModel, Generic[FeatureGeometryType, FeaturePropertyType]):
    example_items: dict[str, FeatureCollection[int, FeatureGeometryType, FeaturePropertyType]]


class Envelope(BaseModel, Generic[FeatureGeometryType, FeaturePropertyType]):
    vectorQuery: VectorQueryContent[FeatureGeometryType, FeaturePropertyType]


class UnusedPayload(BaseModel):
    dead: str | None = None


class Client:
    async def vector_query(
        self,
        raw: object,
        propertiesModel: Type[PayloadType],
    ) -> Envelope[GeometryTypes, PayloadType]:
        adapter = TypeAdapter(Envelope[GeometryTypes | None, propertiesModel])
        parsed = adapter.validate_python(raw)
        return parsed
