from io import BytesIO
from typing import Tuple


class Executor:
    async def run_in_executor(self, pool: object, callback: object):
        return callback()


class Store:
    def get_file(self) -> Tuple[BytesIO, str]:
        return BytesIO(b"pdf"), "file.pdf"


async def respond(loop: Executor, store: Store) -> bytes:
    file_bio, file_name = await loop.run_in_executor(None, store.get_file)
    file_bio.seek(0)
    return file_bio.read() + file_name.encode("utf-8")


respond(Executor(), Store())
