class Router:
    def post(self, path: str):
        def decorate(function):
            return function

        return decorate
