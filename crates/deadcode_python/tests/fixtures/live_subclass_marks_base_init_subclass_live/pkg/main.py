class Base:
    def __init_subclass__(cls) -> None:
        validate_subclass(cls)


class Live(Base):
    pass


class DeadBase:
    def __init_subclass__(cls) -> None:
        validate_subclass(cls)


class DeadChild(DeadBase):
    pass


def validate_subclass(cls: type[object]) -> None:
    cls.__name__


def run() -> type[Live]:
    return Live


run()
