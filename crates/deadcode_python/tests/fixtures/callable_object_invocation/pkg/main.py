class Worker:
    def __call__(self, value: int) -> int:
        return value + 1

    def unused(self) -> int:
        return 0


class UnusedWorker:
    def __call__(self, value: int) -> int:
        return value


def run() -> int:
    worker = Worker()
    return worker(1)


run()
