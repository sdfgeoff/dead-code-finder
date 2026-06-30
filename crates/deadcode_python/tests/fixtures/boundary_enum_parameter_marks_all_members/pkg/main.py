from pkg.framework import Router
from pkg.examples import ExampleRole


router = Router()


@router.post("/{tagname}")
def post_user_tag(tagname: ExampleRole) -> None:
    print(tagname)
