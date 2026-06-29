from api.framework import App
from api.routes import router


app = App()
app.include_router(router)
