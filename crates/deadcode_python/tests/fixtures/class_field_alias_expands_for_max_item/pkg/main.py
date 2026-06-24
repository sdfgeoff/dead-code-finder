from typing import Generic, TypeVar


Id = TypeVar("Id")
Geometry = TypeVar("Geometry")
PropertiesType = TypeVar("PropertiesType")


class Feature(Generic[Id, Geometry, PropertiesType]):
    properties: PropertiesType
    geometry: Geometry


class Properties:
    amount: float
    report_category: str
    credit_scheme: str
    unused: str


CarbonFeature = Feature[int, str | None, Properties]


class CarbonArea:
    features: list[CarbonFeature]


def run(area: CarbonArea) -> tuple[str, str]:
    largest_feature = max(area.features, key=lambda f: f.properties.amount)
    largest_props = largest_feature.properties
    return largest_props.report_category, largest_props.credit_scheme


run(CarbonArea())
