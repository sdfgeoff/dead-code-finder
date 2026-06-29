from pkg.service import shared, tests_only


def test_service_path() -> None:
    tests_only()
    shared()


def dead_test_helper() -> None:
    pass
