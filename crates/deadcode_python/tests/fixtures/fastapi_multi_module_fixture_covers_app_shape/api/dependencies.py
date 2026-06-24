from api.models import ExampleEntity


def get_current_user():
    return ExampleEntity(name="Ada")


def unused_dependency():
    pass
