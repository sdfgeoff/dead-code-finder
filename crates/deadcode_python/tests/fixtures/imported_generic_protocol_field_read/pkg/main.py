from pkg.protocols import LoaderProtocol


class Loader:
    cache_key: str

    def load(self) -> list[str]:
        return ["value"]


def load_key(loader: LoaderProtocol[str]) -> str:
    return loader.cache_key


load_key(Loader())
