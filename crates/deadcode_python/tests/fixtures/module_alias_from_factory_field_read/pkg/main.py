from pkg.config import get_config


Config = get_config()


def provider() -> bool:
    return Config.ENABLE_FEATURE


source_functions = {
    "provider": provider,
}


def run() -> object:
    return source_functions["provider"]


run()
