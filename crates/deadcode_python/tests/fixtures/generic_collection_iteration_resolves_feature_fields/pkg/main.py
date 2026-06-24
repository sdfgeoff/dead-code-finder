from pkg.models import FeatureCollection, Geometry, Properties


def consume(collection: FeatureCollection[int, Geometry, Properties]):
    for feature in collection.features:
        feature.geometry.dump()
        feature.properties.label()


def run():
    consume(FeatureCollection())


run()
