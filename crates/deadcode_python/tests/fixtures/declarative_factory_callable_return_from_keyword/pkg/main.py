from pkg.db import Query, query_titles


def get_titles(session: object, record_id: int) -> list[str]:
    rows = query_titles(session, Query())
    return [
        f"{row.field_code}:{item.strip()}"
        for row in rows
        if row.properties is not None
        for item in row.properties.owners.split(",")
    ]


get_titles(object(), 1)
