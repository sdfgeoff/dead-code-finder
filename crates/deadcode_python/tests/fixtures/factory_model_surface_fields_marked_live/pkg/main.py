from pkg.db import InputRow, get_user


def run() -> str:
    row = get_user(object(), InputRow(user_id=1))
    return row.name


run()
