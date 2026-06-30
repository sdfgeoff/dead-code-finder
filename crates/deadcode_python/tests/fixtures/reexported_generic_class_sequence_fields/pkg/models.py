from pkg import Feature
from pkg.main import Properties


ValueFeature = Feature[Properties]


class ValueArea:
    features: list[ValueFeature]


class AreaWithScheme:
    area: ValueArea
