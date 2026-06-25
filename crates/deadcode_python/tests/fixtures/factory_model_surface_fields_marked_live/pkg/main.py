from pkg.db import InputRow, PositionalInput, get_associations, get_user


def run() -> str:
    row = get_user(object(), InputRow(user_id=1))
    associations = get_associations(
        object(), PositionalInput(user_id=1, record_id=row.id)
    )
    for association in associations:
        return f"{row.name}:{association.group_id}"
    return row.name


run()
