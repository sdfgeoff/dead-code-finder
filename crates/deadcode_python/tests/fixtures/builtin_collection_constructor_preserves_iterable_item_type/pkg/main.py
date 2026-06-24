from typing import Generic, Sequence, TypeVar


T = TypeVar("T")


class Feature(Generic[T]):
    properties: T
    geometry: str
    unused: int


class Properties:
    status: str
    unused: int


def summarize(features: Sequence[Feature[Properties]]) -> list[str]:
    feature_list = list(features)
    return [f"{feature.geometry}:{feature.properties.status}" for feature in feature_list]


summarize([Feature[Properties]()])
