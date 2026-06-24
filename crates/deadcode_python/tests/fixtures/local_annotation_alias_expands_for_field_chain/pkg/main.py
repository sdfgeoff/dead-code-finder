from typing import Generic, TypeVar


T = TypeVar("T")


class Item:
    live: str
    unused: str

    def model_dump(self) -> dict[str, object]:
        return {}


class Box(Generic[T]):
    item: T


ItemBox = Box[Item]


def run(source: Item) -> object:
    box: ItemBox = Box(item=source)
    data = box.item.model_dump()
    data.update({"extra": 1})
    return data


run(Item())
