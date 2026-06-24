from pkg import FeatureCollection
from pkg.core import BoundaryProperties


BoundaryFeatureCollection = FeatureCollection[int, str | None, BoundaryProperties | None]
