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


def zipped_area(features: Sequence[Feature[Properties]]) -> float:
    feature_list = list(features)
    areas = [1.0 for feature in feature_list]
    rounded_areas = [2.0 for feature in feature_list]
    results = [
        rounded_area if feature.properties.amount > 0 else area
        for feature, area, rounded_area in zip(feature_list, areas, rounded_areas)
    ]
    return results[0]


total_area([])
flattened_area([])
zipped_area([])
