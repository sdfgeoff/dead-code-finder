class Config:
    ENABLE_FEATURE: bool = True
    UNUSED_FEATURE: bool = False


config = Config()


def run() -> bool:
    return not config.ENABLE_FEATURE


run()
