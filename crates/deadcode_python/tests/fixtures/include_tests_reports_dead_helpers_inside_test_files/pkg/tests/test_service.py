import pytest
from pytest import fixture


def test_live():
    pass


def dead_helper():
    pass


class TestCollected:
    def test_method(self):
        pass

    def helper_method(self):
        pass


class HelperClass:
    def test_like_method_on_non_test_class(self):
        pass


@pytest.fixture(autouse=True)
def automatic_fixture():
    pass


@pytest.fixture
def direct_fixture():
    pass


@pytest.fixture(name="renamed_fixture")
def aliased_fixture():
    pass


@fixture()
def dependent_fixture(direct_fixture):
    pass


def test_uses_fixtures(dependent_fixture, renamed_fixture):
    pass


@pytest.fixture
def unused_fixture():
    pass


def build_test_data():
    return "live"


TEST_DATA = [build_test_data()]


def test_reads_module_value():
    assert TEST_DATA
