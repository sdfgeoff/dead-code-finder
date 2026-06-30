def helper() -> str:
    return "kept"


# dead-code-finder: allow
def api_surface() -> str:
    return helper()


def dead() -> str:
    return "dead"
