class Config:
    ENABLE_FEATURE: bool = True
    UNUSED_FEATURE: bool = False


def get_config() -> Config:
    return Config()
