def read_config(path: str):
    with open(path, "r") as file:
        return file.read()


read_config("settings.json")
