class Parsed:
    method: str
    url: str
    unused: str


def parse(line: str) -> Parsed:
    _ip_and_port, rest = line.split(' - "', 1)
    middle, response_code = rest.rsplit('" ', 1)
    method, url_and_http = middle.split(" ", 1)
    url, _http_version = url_and_http.rsplit(" HTTP/", 1)
    if response_code.strip().startswith("2"):
        return Parsed(method=method, url=url.replace("a", "b"))
    return Parsed(method=method, url=url)


def run() -> str:
    parsed = parse('127.0.0.1:1234 - "GET / HTTP/1.1" 200')
    return parsed.method + parsed.url


run()
