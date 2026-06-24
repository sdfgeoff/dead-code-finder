class ExampleItem:
    name: str
    tile_url: str
    unused: str


def build_example_items() -> list[ExampleItem]:
    example_items = []
    example_items.append(ExampleItem(name="Planet", tile_url="https://example.test/{z}/{x}/{y}.png"))
    return example_items


build_example_items()
