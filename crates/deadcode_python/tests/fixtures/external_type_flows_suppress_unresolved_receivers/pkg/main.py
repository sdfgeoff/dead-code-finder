from pkg.client import Client
from pkg.logging_flow import run_logging_flow


def run():
    run_logging_flow()
    client = Client()
    client.run()


run()
