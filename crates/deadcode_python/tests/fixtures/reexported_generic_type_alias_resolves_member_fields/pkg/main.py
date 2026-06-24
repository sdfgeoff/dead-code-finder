from pkg.models import BoundaryFeatureCollection


def process(boundary: BoundaryFeatureCollection):
    total = 0.0
    for feature in boundary.features:
        total += feature.properties.title_area
        geometry = feature.geometry
    return total, geometry


process(None)
