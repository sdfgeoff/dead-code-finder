from pkg import Feature
from pkg.main import Properties


CarbonFeature = Feature[Properties]


class CarbonArea:
    features: list[CarbonFeature]


class AreaWithScheme:
    area: CarbonArea
