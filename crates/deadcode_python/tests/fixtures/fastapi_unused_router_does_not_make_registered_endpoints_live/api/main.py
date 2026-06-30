from fastapi import FastAPI

from api.live_routes import router as live_router
from api import unused_routes

app = FastAPI()
app.include_router(live_router)
