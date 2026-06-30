from fastapi import APIRouter

router = APIRouter()


@router.get("/live")
def live_endpoint() -> None:
    live_helper()


def live_helper() -> None:
    pass
