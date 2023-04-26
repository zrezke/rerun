from enum import Enum
from typing import Union


class Topic(Enum):
    """All topics that can be subscribed to."""

    ColorImage = 0
    LeftMono = 1
    RightMono = 2
    DepthImage = 3
    PointCloud = 4
    PinholeCamera = 5
    Rectangle = 6
    Rectangles = 7
    ImuData = 8

    @classmethod
    def create(cls, name_or_id: Union[str, int]) -> "Topic":
        if type(name_or_id) == str:
            return Topic[name_or_id]
        elif type(name_or_id) == int:
            return Topic(name_or_id)
        else:
            raise ValueError("Invalid topic name or id: ", name_or_id)
