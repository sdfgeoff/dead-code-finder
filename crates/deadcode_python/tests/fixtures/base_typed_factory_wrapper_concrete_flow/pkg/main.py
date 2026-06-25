from pkg.providers import create_connection


def run() -> None:
    connection = create_connection()
    connection.lookup("live")


run()
