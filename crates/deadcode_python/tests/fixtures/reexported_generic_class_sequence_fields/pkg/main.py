from typing import Sequence, TypeVar

from pkg import Feature
from pkg.models import AreaWithScheme
from pkg.util import flatten

PropertiesType = TypeVar("PropertiesType")


class Properties:
    amount: float
    unused: int


class Overlap:
    amount: float
    properties: Properties


StatsType = TypeVar("StatsType")


class Stats:
    amount: float
    unused: int


class DumpSource:
    keep: str
    skip: str


class DumpTarget:
    keep: str
    skip: str
    extra: int


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


def optional_nested_overlap(overlap_groups: list[list[Overlap]] | None) -> float | None:
    if overlap_groups is None:
        return None
    results = [
        overlap.amount
        for feature, overlap_group in zip([], overlap_groups)
        for overlap in overlap_group
    ]
    return results[0]


def stats_from_payload(stats_cls: type[StatsType], payload: dict[str, object]) -> StatsType:
    return stats_cls(**payload, amount=1.0)


def target_from_dump(source: DumpSource) -> DumpTarget:
    return DumpTarget(**source.model_dump(exclude={"skip"}), extra=1)


total_area([])
flattened_area([])
zipped_area([])
optional_nested_overlap([])
stats_from_payload(Stats, {})
target_from_dump(DumpSource())
