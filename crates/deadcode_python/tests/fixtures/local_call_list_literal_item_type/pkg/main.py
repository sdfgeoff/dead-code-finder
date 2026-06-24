from typing import Generic, TypeVar


TProperties = TypeVar("TProperties")


class Properties:
    label: str
    unused: str


class Feature(Generic[TProperties]):
    properties: TProperties
    geometry: str
    unused: str


def dissolve() -> Feature[Properties]:
    return Feature()


def run() -> list[str]:
    features = [dissolve()]
    return [f"{item.properties.label}:{item.geometry}" for item in features]


run()
