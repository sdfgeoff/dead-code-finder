from pkg import format_template
from pkg.first import FirstTemplate


def run() -> str:
    return format_template(FirstTemplate(label="Live"))


run()
