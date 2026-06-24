class Scheme:
    name: str
    enabled: bool
    unused: str


PRIMARY = Scheme()
SECONDARY = Scheme()

SCHEMES = [PRIMARY]
SCHEMES.append(SECONDARY)
