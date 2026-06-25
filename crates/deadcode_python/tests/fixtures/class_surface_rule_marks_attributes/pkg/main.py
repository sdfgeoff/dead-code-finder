from pkg.orm import Base, column


class LiveModel(Base):
    __tablename__ = "live"
    id = column()
    used = column()


class DeadModel(Base):
    __tablename__ = "dead"
    id = column()


def run() -> LiveModel:
    return LiveModel()


run()
