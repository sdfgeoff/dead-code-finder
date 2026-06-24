class DomainError(Exception):
    message: str
    unused: str


def run() -> str:
    try:
        raise DomainError()
    except DomainError as error:
        return error.message


run()
