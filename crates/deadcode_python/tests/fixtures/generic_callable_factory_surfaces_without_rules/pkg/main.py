from pkg.db import (
    BatchInput,
    InputRow,
    get_many as db_get_many,
    get_one as db_get_one,
    local_query_optional,
    save_batch as db_save_batch,
)


def run() -> str:
    row = db_get_one(object(), InputRow(required=1, serialized_only="yes"))
    db_save_batch(object(), [BatchInput(item_id=1, payload="value")])
    many = db_get_many(object(), InputRow(required=2, serialized_only="yes"))
    labels = [item.constructed_only for item in db_get_many(object(), InputRow(required=4, serialized_only="yes"))]
    local = local_query_optional(input=InputRow, output=BatchInput, sql="SELECT item_id, payload")
    local_row = local(object(), InputRow(required=3, serialized_only="yes"))
    if row is None or local_row is None:
        return "missing"
    return f"{row.id}:{many[0].constructed_only}:{labels[0]}:{local_row.item_id}"


run()
