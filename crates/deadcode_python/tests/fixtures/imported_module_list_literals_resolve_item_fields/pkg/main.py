from pkg.schemes import SCHEMES


def run():
    for scheme in SCHEMES:
        if scheme.enabled:
            return scheme.name
    return None


run()
