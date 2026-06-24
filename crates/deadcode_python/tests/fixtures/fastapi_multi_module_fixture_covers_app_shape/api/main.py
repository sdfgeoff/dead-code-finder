from fastapi import FastAPI

from api.routes.loader import include_routes

app = FastAPI()
include_routes(app)
