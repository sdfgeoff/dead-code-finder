from typing import Generic, TypeVar


T = TypeVar("T")


class Feature(Generic[T]):
    properties: T


class FeatureCollection(Generic[T]):
    features: list[Feature[T]]


class Properties:
    amount: float
    report_category: str
    credit_scheme: str
    unused: str


def run(area: FeatureCollection[Properties]) -> tuple[str, str]:
    largest_feature = max(area.features, key=lambda f: f.properties.amount)
    largest_props = largest_feature.properties
    return largest_props.report_category, largest_props.credit_scheme


run(FeatureCollection())
