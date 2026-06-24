from pathlib import Path


GUIDE_PATH = Path(__file__).resolve().parent.parent / "knowledge" / "guide.md"
GUIDE_CONTENT = GUIDE_PATH.read_text(encoding="utf-8") if GUIDE_PATH.exists() else ""


def guide() -> str:
    return GUIDE_CONTENT


guide()
