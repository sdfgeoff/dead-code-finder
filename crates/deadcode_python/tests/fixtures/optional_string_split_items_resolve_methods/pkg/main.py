class Properties:
    owners: str | None
    unused: int


def owner_list(properties: Properties) -> list[str]:
    return (
        []
        if properties.owners is None
        else [item.strip() for item in properties.owners.split(",")]
    )


owner_list(Properties())
