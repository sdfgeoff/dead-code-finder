from pkg.logic.entities import all_tags


def run() -> list[str]:
    return [example_tag.value for example_tag in all_tags()]


run()
