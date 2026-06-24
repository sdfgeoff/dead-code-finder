from typing import Any


class Client:
    def __init__(self, service: Any):
        self.service: Any = service

    def fetch(self):
        return self.service.files().list().execute()

    def folder_id(self) -> str | None:
        response = self.fetch()
        folders = response.get("files", [])
        if folders:
            return folders[0].get("id")
        return response.get("nextPageToken")


Client(object()).folder_id()
