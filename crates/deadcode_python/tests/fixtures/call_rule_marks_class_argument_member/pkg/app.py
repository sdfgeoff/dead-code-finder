from pkg.framework import Application
from pkg.middleware import LoggedMiddleware


app = Application()


def run() -> None:
    app.add_middleware(LoggedMiddleware)


run()
