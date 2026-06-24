class Heading:
    stock_heading: str
    residual_heading: str
    unused: str


HEADINGS = {
    "pine": Heading(),
    "fir": Heading(),
}


def load():
    for category, heading in HEADINGS.items():
        stock = heading.stock_heading
        residual = heading.residual_heading
    return stock, residual, category


load()
