from fastapi import APIRouter, Depends, FastAPI
from pydantic import BaseModel

app = FastAPI()
router = APIRouter()


class ExampleEntity(BaseModel):
    name: str
    age: int


def get_user():
    return ExampleEntity(name="Ada")


def unused_dependency():
    pass


@router.get("/entities")
def list_users(entity=Depends(get_user)):
    pass


app.include_router(router)
