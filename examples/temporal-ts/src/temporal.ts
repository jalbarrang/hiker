// System-under-test for the hiker TypeScript worked example.
//
// Correspondence is BY CONVENTION: the exported function names and the object
// field names must match `.hiker/temporal.tent`. The generated fast-check
// test (in .hiker-cache/ts/) imports this module and checks each relation
// against its law.
//
// Toggle the bug with the env var HIKER_BUGGY=1.

export interface TemporalPoint {
  media: number;
  t: number;
}

export interface TemporalInterval {
  media: number;
  t0: number;
  t1: number;
}

const BUGGY = process.env.HIKER_BUGGY === "1";

// Two intervals overlap iff they share a media item and their ranges intersect.
export function temporal_overlap(a: TemporalInterval, b: TemporalInterval): boolean {
  return a.media === b.media && a.t0 <= b.t1 && b.t0 <= a.t1;
}

// A point lies inside an interval iff they share a media item and the point's
// instant is within [t0, t1].
export function point_in_interval(p: TemporalPoint, i: TemporalInterval): boolean {
  if (BUGGY) {
    // The video's bug: give the point a phantom [t, t+5] duration and reuse the
    // interval-overlap machinery. Looks reasonable, passes naive tests, but it
    // has quietly turned a point into a range.
    const degenerate: TemporalInterval = { media: p.media, t0: p.t, t1: p.t + 5 };
    return temporal_overlap(degenerate, i);
  }
  return p.media === i.media && i.t0 <= p.t && p.t <= i.t1;
}
