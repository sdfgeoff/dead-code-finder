from pkg.first import FirstTemplate
from pkg.second import SecondTemplate
from pkg.third import ThirdTemplate

AnyTemplate = FirstTemplate | SecondTemplate | ThirdTemplate


def format_template(template: AnyTemplate) -> str:
    return template.template_type
