class Sheet:
    label: str
    unused: str


class Workbook:
    @property
    def worksheets(self) -> list[Sheet]:
        return []


def load(book: Workbook):
    for sheet in book.worksheets:
        return sheet.label
    return None


load(Workbook())
