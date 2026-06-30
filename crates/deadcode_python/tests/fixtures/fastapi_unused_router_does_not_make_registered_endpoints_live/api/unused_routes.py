from fastapi import APIRouter

router = APIRouter()


@router.get("/unused")
def unused_endpoint() -> None:
    unused_helper()


def unused_helper() -> None:
    pass
