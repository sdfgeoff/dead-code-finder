class Item:
    name: str
    unused: str


class Store:
    items: dict[str, Item]


def run(store: Store):
    item = store.items.get("primary")
    if item is None:
        return None
    return item.name


run(Store())
