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


class Bounds:
    center: float

    def distance_between_centers(self, other: "Bounds") -> float:
        return self.center - other.center

    def unused_method(self) -> float:
        return self.center


class SceneAsset:
    bound: Bounds


def run(area: FeatureCollection[Properties]) -> tuple[str, str]:
    largest_feature = max(area.features, key=lambda f: f.properties.amount)
    largest_props = largest_feature.properties
    return largest_props.report_category, largest_props.credit_scheme


def choose_asset(scene_assets: list[SceneAsset], bounds: Bounds) -> SceneAsset:
    return min(
        scene_assets,
        key=lambda asset: bounds.distance_between_centers(asset.bound),
    )


run(FeatureCollection())
choose_asset([SceneAsset()], Bounds())
