from fastapi import APIRouter

router = APIRouter()


@router.get("/entities")
def list_users():
    pass
