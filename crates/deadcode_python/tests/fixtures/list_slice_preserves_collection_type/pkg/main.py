def used(text: str) -> str:
    name = text.split("/")[::-1][0].split(".")[0]
    return name.split("_")[0]


def dead() -> None:
    pass


used("a/b_c.txt")
