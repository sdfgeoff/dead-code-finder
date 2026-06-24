from pkg.client import Client
from pkg.logging_flow import run_logging_flow, run_structlog_flow


def run():
    run_logging_flow()
    run_structlog_flow()
    client = Client()
    client.run()


run()
