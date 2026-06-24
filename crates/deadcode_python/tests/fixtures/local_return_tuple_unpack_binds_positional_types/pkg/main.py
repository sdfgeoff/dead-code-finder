from typing import Tuple


class ExampleRef:
    example_item_id: int
    version_id: int
    unused: int


class Properties:
    name: str
    unused: str


def insert_example_item() -> Tuple[ExampleRef, Properties]:
    return ExampleRef(), Properties()


def run() -> tuple[int, str]:
    example_item_ref, properties = insert_example_item()
    return example_item_ref.version_id, properties.name


run()
