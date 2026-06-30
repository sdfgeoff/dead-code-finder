class ExampleMessage:
    field_uuid: str
    resource_id: int
    field_old: str
    field_new: str
    field_unused: str


def render(data: ExampleMessage) -> str:
    return "Record {data.field_uuid} in resource {data.resource_id}: {data.field_old} -> {data.field_new}".format(
        data=data
    )


def main() -> None:
    note = ExampleMessage()
    note.field_uuid = "item-1"
    note.resource_id = 7
    note.field_old = "open"
    note.field_new = "done"
    print(render(note))


main()
