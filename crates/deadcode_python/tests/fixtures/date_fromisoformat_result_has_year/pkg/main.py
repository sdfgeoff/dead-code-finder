from datetime import date


def get_year(value: str) -> int:
    parsed = date.fromisoformat(value)
    return parsed.year


get_year("2024-01-01")
