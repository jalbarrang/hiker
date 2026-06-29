"""System-under-test for the rascador Python worked example.

Correspondence is BY CONVENTION: the class names, their fields, and the
function names must match `.rascador/temporal.tent`. The generated Hypothesis
test (in .rascador-cache/python/) imports this module and checks each relation
against its law.

Toggle the bug with the env var RASCADOR_BUGGY=1.
"""

import os
from dataclasses import dataclass


@dataclass
class TemporalPoint:
    media: int
    t: int


@dataclass
class TemporalInterval:
    media: int
    t0: int
    t1: int


_BUGGY = os.environ.get("RASCADOR_BUGGY") == "1"


def temporal_overlap(a, b):
    """Two intervals overlap iff same media item and ranges intersect."""
    return a.media == b.media and a.t0 <= b.t1 and b.t0 <= a.t1


def point_in_interval(p, i):
    """A point lies inside an interval iff same media item and t in [t0, t1]."""
    if _BUGGY:
        # The video's bug: give the point a phantom [t, t+5] duration and reuse
        # the interval-overlap machinery. Looks reasonable, passes naive tests,
        # but it has quietly turned a point into a range.
        degenerate = TemporalInterval(media=p.media, t0=p.t, t1=p.t + 5)
        return temporal_overlap(degenerate, i)
    return p.media == i.media and i.t0 <= p.t and p.t <= i.t1
