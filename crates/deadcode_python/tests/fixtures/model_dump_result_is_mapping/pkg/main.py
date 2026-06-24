class Model:
    value: str


class Container:
    model: Model


def run(container: Container) -> object:
    data = container.model.model_dump()
    data.update({"extra": 1})
    return data


run(Container())
