class ExampleItem:
    example_item_id: int
    version_id: int
    unused: int


class ExampleCollection:
    example_items: list[ExampleItem]
    unused: str


def etag(example_item_list: ExampleCollection) -> bytes:
    example_item_summary = "".join(
        [f"/{example_item.example_item_id}@{example_item.version_id}" for example_item in example_item_list.example_items]
    )
    return example_item_summary.encode("utf-8")


etag(ExampleCollection())
