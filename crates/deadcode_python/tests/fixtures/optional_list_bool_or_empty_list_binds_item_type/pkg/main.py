class BaseProperties:
    class_labels: list[str] | None


class Properties(BaseProperties):
    unused: int


def labels(properties: Properties) -> list[str]:
    class_labels = properties.class_labels or []
    return [label.lower() for label in class_labels]


labels(Properties())
