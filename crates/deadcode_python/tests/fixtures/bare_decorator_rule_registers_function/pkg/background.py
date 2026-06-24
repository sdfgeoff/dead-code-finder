from collections.abc import Callable


class WrapperResource:
    def __enter__(self):
        pass

    def __exit__(self):
        pass


class DeadWrapperResource:
    def __enter__(self):
        pass

    def __exit__(self):
        pass


def make_background_task(func: Callable[..., None]):
    def wrapper(*args, **kwargs):
        with WrapperResource():
            func(*args, **kwargs)

    return wrapper
