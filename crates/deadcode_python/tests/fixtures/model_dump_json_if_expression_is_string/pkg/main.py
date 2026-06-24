import hashlib
from pydantic import BaseModel


class Geometry(BaseModel):
    unused: str


class Feature:
    geometry: Geometry | None


class GeometryHash:
    geometry_hash: str
    unused: str


def from_feature(feature: Feature) -> GeometryHash:
    geometry_json = (
        "null" if feature.geometry is None else feature.geometry.model_dump_json()
    )
    return GeometryHash(geometry_hash=hashlib.md5(geometry_json.encode()).hexdigest())


from_feature(Feature())
