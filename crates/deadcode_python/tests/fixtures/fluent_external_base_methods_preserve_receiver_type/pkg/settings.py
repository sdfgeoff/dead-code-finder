from pydantic import BaseModel


class Config(BaseModel):
    host: str = "example.test"
    port: int = 443
    unused: str = "dead"


DevelopmentConfig = Config().model_copy(update={"host": "dev.example.test"})
PytestConfig = DevelopmentConfig.model_copy(update={"port": 8000})
ValidatedConfig = Config.model_validate({"host": "validated.example.test", "port": 8443})


def run(config: Config):
    return config.host, ValidatedConfig.port
