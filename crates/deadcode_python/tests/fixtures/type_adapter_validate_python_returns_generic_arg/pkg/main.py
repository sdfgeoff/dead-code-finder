from pkg.models import EventAdapter

def run(payload: object) -> str:
    event = EventAdapter.validate_python(payload)
    return event.source_id


run({})
