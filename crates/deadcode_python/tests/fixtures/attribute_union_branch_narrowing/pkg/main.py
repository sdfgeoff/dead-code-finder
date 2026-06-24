class RegionalStock:
    region: str
    values: list[float]
    unused: str


class GlobalStock:
    values: list[float]
    unused: str


class Species:
    stock: list[RegionalStock] | GlobalStock | None
    unused: str


def convert(species: Species) -> dict[str, list[float]]:
    if species.stock is None:
        return {}
    elif isinstance(species.stock, GlobalStock):
        return {"global": species.stock.values}
    else:
        return {stock.region: stock.values for stock in species.stock}


def summarize(species: Species) -> list[str] | str:
    return (
        "ALL"
        if isinstance(species.stock, GlobalStock) or species.stock is None
        else [stock.region for stock in species.stock]
    )


def run(species: Species):
    convert(species)
    summarize(species)


run(Species())
