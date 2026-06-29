import pytest

from api.app import app
from api.routes import get_service


class FakeService:
    async def run(self) -> str:
        return "fake"

    def unused(self) -> str:
        return "dead"


@pytest.fixture
def fake_service() -> FakeService:
    service = FakeService()
    app.dependency_overrides[get_service] = lambda: service
    return service


def test_route_uses_override(fake_service: FakeService) -> None:
    assert fake_service is not None
