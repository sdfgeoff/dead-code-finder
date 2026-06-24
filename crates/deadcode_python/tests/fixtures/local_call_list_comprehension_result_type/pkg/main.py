class Dimensions:
    category: str
    unused: str


class DimsAndMeasures:
    dimensions: Dimensions
    unused: str


class ExampleInput:
    category: str
    amount: float


def make_dims_and_measures(category: str) -> DimsAndMeasures:
    return DimsAndMeasures(dimensions=Dimensions(category=category))


def create_unknown() -> DimsAndMeasures:
    return DimsAndMeasures(dimensions=Dimensions(category="unknown"))


def run() -> list[str]:
    results = [
        make_dims_and_measures(example_input.category)
        for example_input in [
            ExampleInput(category="native", amount=1.0),
            ExampleInput(category="exotic", amount=0.0),
        ]
        if example_input.amount > 0
    ]
    results.append(create_unknown())
    return [dims.dimensions.category for dims in results]


run()
