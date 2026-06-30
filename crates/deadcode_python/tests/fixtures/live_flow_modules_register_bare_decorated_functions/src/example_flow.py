from prefect import flow


@flow(name="example_flow")
def example_flow() -> None:
    helper()


def helper() -> None:
    pass


def dead_helper() -> None:
    pass
