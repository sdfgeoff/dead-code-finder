from lib.orchestrator import orchestrate, stream_joiner

from app.context import ExampleContext


def run(resource_id: int) -> str:
    frontend_context = ExampleContext(resource_id=resource_id)
    return stream_joiner(lambda: orchestrate(frontend_context=frontend_context))


run(1)
