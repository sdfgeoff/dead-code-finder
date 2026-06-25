from pkg.models import Client, Payload


async def run_through_generic_helper(raw: object) -> str:
    parsed = await Client().vector_query(raw, propertiesModel=Payload)
    example_item = parsed.vectorQuery.example_items["0"]
    return example_item.features[0].properties.name


async def main() -> None:
    await run_through_generic_helper({})


def entrypoint() -> None:
    import asyncio

    asyncio.run(main())


entrypoint()
