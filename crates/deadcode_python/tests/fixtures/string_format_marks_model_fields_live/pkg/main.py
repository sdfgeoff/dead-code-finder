class Notification:
    task_uuid: str
    resource_id: int
    old_status: str
    new_status: str
    unused_field: str


def render(data: Notification) -> str:
    return "ExampleItem {data.task_uuid} on resource {data.resource_id}: {data.old_status} -> {data.new_status}".format(
        data=data
    )


def main() -> None:
    note = Notification()
    note.task_uuid = "item-1"
    note.resource_id = 7
    note.old_status = "open"
    note.new_status = "done"
    print(render(note))


main()
