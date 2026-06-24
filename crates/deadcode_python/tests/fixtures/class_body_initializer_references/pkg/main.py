def parse_csv(raw: str) -> list[str]:
    return raw.split(",")


def unused_parse(raw: str) -> list[str]:
    return raw.split("|")


class Config:
    pipeline_ids: list[str] = parse_csv("one,two")
    feature_enabled = bool(pipeline_ids)


def run() -> list[str]:
    return Config.pipeline_ids


run()
