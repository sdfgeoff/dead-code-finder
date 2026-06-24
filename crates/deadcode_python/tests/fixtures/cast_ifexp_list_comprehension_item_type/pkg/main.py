from typing import List, cast


class Polygon:
    bounds: tuple[float, float, float, float]
    unused: str


class MultiPolygon:
    geom_type: str
    geoms: list[Polygon]


def run(covering_area: Polygon | MultiPolygon) -> list[tuple[float, float, float, float]]:
    polygons = (
        [covering_area]
        if covering_area.geom_type == "Polygon"
        else cast(List[Polygon], list(covering_area.geoms))
    )
    return [poly.bounds for poly in polygons]


run(Polygon())
