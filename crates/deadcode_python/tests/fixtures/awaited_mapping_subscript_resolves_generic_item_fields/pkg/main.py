from pkg.models import Properties, Response


class Client:
    async def vector_query(self) -> Response[dict, Properties]:
        return Response()


async def run(client: Client):
    survey_data = await client.vector_query()
    example_item_data = survey_data.vector_query.example_items["example_item"]
    for feature in example_item_data.features:
        return feature.geometry, feature.properties.name


async def main():
    await run(Client())


main()
