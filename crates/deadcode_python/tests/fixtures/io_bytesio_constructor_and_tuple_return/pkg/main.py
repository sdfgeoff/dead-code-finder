from io import BytesIO
from typing import Tuple


def make_file() -> Tuple[BytesIO, str]:
    import io

    buffer = io.BytesIO(b"pdf")
    buffer.seek(0)
    return buffer, "file.pdf"


def respond() -> bytes:
    file_bio, file_name = make_file()
    file_bio.seek(0)
    return file_bio.read() + file_name.encode("utf-8")


respond()
