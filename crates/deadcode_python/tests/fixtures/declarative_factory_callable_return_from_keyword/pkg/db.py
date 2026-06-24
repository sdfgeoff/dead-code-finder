from pkg.store import make_query


class Query:
    record_id: int


class TitleProperties:
    owners: str
    unused: int


class TitleRow:
    field_code: str
    properties: TitleProperties | None
    unused: int


query_titles = make_query(output=TitleRow, sql="select field_code, properties")
