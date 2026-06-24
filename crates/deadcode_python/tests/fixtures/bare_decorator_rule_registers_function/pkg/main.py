from pkg import background


class Resource:
    def __enter__(self):
        pass

    def __exit__(self):
        pass


class DeadResource:
    def __enter__(self):
        pass

    def __exit__(self):
        pass


@background.make_background_task
def run_task():
    with Resource():
        pass
