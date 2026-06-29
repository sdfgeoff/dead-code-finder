from pydantic import BaseModel


class Delivery(BaseModel):
    primary: list[str]
    copy: list[str] = []
    blind_copy: list[str] = []


class Content(BaseModel):
    charset: str = "UTF-8"
    data: str


class Message(BaseModel):
    subject: Content
    body: Content


class Request(BaseModel):
    delivery: Delivery
    message: Message


def send_to_external(payload: object) -> None:
    pass


def run() -> None:
    request = Request(
        delivery=Delivery(primary=["person@example.com"]),
        message=Message(
            subject=Content(data="Hello"),
            body=Content(data="Body"),
        ),
    )
    send_to_external(request.model_dump())


run()
