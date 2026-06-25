from typing import Any


class ExternalWorkbook:
    worksheets: list[Any]


class CompatCell:
    def __init__(self) -> None:
        self._value = "ok"

    @property
    def value(self) -> str:
        return self._value


class CompatSheet:
    label: str

    def cell(self, row: int, col: int) -> CompatCell:
        return CompatCell()


class CompatWorkbook:
    worksheets: list[CompatSheet]

    def __getitem__(self, name: str) -> CompatSheet:
        return CompatSheet()


def load(book: ExternalWorkbook | CompatWorkbook) -> str:
    sheet = book["Sheet1"]
    return sheet.cell(1, 1).value


def run() -> str:
    return load(CompatWorkbook())


run()
