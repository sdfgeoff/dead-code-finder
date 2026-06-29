from app.realtime import publish_event
from app.web import read_value, write_value


def run() -> None:
    write_value()
    read_value()
    publish_event()


run()
