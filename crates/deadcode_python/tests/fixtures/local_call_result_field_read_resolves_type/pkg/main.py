class Settings:
    host: str
    unused: str


def get_settings() -> Settings:
    return Settings()


def run() -> str:
    host = get_settings().host
    if host.startswith("localhost:"):
        return host.replace("localhost", "127.0.0.1")
    return host


run()
