class ExampleEntity:
    def save(self):
        pass


class Other:
    def save(self):
        pass


class Box[T]:
    value: T


def process(box: Box[ExampleEntity]):
    entity = box.value
    entity.save()


def unresolved(x):
    x.save()


if __name__ == "__main__":
    process(Box())
    unresolved(None)
