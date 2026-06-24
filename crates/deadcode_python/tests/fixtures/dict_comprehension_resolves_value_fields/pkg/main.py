class Sheet:
    label: str
    unused: str


class Book:
    worksheets: list[Sheet]


def heading_for(sheet: Sheet) -> str:
    return sheet.label


def load(sheets: list[Sheet]):
    sheets_by_heading = {heading_for(sheet): sheet for sheet in sheets}
    selected = sheets_by_heading.get("stock")
    if selected is None:
        return None
    return selected.label


def load_from_book(book: Book):
    sheets = book.worksheets
    sheets_by_heading = {heading_for(sheet): sheet for sheet in sheets}
    selected = sheets_by_heading.get("stock")
    if selected is None:
        return None
    return selected.label


load([])
