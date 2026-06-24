from typing import Optional


class Branding:
    logo_url: str
    unused: str


class Settings:
    branding: Optional[Branding]
    unused: str


def normalize(settings: Settings) -> Branding:
    branding = settings.branding or Branding()
    return Branding(
        logo_url=branding.logo_url
        if branding.logo_url and branding.logo_url.strip() != ""
        else "default"
    )


def run(settings: Settings) -> str:
    return normalize(settings).logo_url


run(Settings())
