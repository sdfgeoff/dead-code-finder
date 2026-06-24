from pkg.factory import make_client


client = make_client


if __name__ == "__main__":
    client().create_example_item()
