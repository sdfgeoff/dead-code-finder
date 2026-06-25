from pkg.models.record import ExampleRole


def all_tags() -> list[ExampleRole]:
    return [example_tag for example_tag in ExampleRole]
