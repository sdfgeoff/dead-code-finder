class ExampleItem:
    example_item_id: int
    version_id: int
    unused: str


class Spec:
    example_item_id: int
    version_id: int


def load_example_items() -> list[ExampleItem]:
    return []


def choose() -> Spec | None:
    example_items = load_example_items()
    if len(example_items) == 0:
        return None
    elif len(example_items) == 1:
        example_item = example_items[0]
    else:
        sorted_example_items = sorted(example_items, key=lambda item: item.example_item_id)
        example_item = sorted_example_items[-1]

    return Spec(example_item_id=example_item.example_item_id, version_id=example_item.version_id)


choose()
