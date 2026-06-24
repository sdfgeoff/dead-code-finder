from pkg.service import Model, get_client


def main():
    result = get_client().fetch(model=Model)
    item = result.item
    return item.field


main()
