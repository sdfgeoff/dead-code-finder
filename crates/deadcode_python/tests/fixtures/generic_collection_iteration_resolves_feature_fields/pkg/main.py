from typing import List

from pkg import FeatureCollection, GeometryType, IdType, MetadataType, PropertyType


def consume(
    collections: List[FeatureCollection[IdType, GeometryType, PropertyType, MetadataType]],
):
    for collection in collections:
        for feature in collection.features:
            feature.geometry.dump()
            feature.properties.label()


def run():
    consume([FeatureCollection()])


run()
