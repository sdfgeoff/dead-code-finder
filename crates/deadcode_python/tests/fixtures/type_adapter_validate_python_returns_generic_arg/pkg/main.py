from pkg.models import EventAdapter, parse_external

def run(payload: object) -> str:
    event = EventAdapter.validate_python(payload)
    external = parse_external(payload)
    return event.source_id + external.item.used_external


run({})
