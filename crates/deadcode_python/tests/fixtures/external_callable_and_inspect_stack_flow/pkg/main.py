import inspect
from framework import RequestResponseEndpoint


callback: RequestResponseEndpoint


def dispatch(call_next: RequestResponseEndpoint) -> None:
    response = call_next(None)
    response.status_code


def log_stack() -> None:
    callstack_reduced = inspect.stack(0)[1:]
    callstack_names = [frame.function for frame in callstack_reduced]
    callstack_names.reverse()


dispatch(callback)
log_stack()
