from io import BytesIO
from typing import Tuple, Union


def make_file() -> Tuple[BytesIO, str]:
    import io

    buffer = io.BytesIO(b"pdf")
    buffer.seek(0)
    return buffer, "file.pdf"


def respond() -> bytes:
    file_bio, file_name = make_file()
    file_bio.seek(0)
    return file_bio.read() + file_name.encode("utf-8")


def maybe_file() -> Union[Tuple[BytesIO, str], None]:
    return make_file()


def respond_optional() -> bytes:
    image = maybe_file()
    if image is None:
        raise RuntimeError("missing")
    b_io, mime = image
    b_io.seek(0)
    return b_io.read() + mime.encode("utf-8")


respond()
respond_optional()
