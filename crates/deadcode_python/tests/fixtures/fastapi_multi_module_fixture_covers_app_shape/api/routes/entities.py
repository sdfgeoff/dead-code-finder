from fastapi import APIRouter, Depends

from api.dependencies import get_current_user

router = APIRouter()


@router.get("/entities/me")
def read_user(entity=Depends(get_current_user)):
    pass
