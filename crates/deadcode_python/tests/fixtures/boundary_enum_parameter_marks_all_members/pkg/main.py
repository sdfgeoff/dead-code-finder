from pkg.framework import Router
from pkg.examples import UserTagEnum


router = Router()


@router.post("/{tagname}")
def post_user_tag(tagname: UserTagEnum) -> None:
    print(tagname)
