from typing import Sequence, TypeVar

from pkg import Feature
from pkg.models import AreaWithScheme
from pkg.util import flatten

PropertiesType = TypeVar("PropertiesType")


class Properties:
    amount: float
    unused: int


def total(features: Sequence[Feature[PropertiesType]]) -> list[PropertiesType]:
    feature_list = list(features)
    return [feature.properties for feature in feature_list]


def total_area(features: Sequence[Feature[Properties]]) -> float:
    results = total(features)
    return results[0].amount


def flattened_area(areas: list[AreaWithScheme]) -> float:
    nested_features = [area.area.features for area in areas]
    features = flatten(nested_features)
    areas = [feature.properties.amount for feature in features]
    return areas[0]


total_area([])
flattened_area([])
