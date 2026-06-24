from typing import Optional


class Actions:
    enabled: bool
    unused: bool


class Settings:
    actions: Optional[Actions]
    notifications: Actions | None
    unused: str


def get_settings(enabled: bool) -> Optional[Settings]:
    return Settings() if enabled else None


def run() -> bool:
    settings = get_settings(True) if True else None
    actions = settings.actions if settings and settings.actions else Actions()
    if settings and settings.notifications:
        return actions.enabled or settings.notifications.enabled
    return actions.enabled


run()
