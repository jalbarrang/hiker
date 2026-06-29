//! System-under-test for the hiker worked example.
//!
//! Correspondence is BY CONVENTION: the struct names, field names, and function
//! names here must match `.hiker/temporal.tent`. The generated proptest in
//! `tests/generated.rs` constructs these structs and calls these functions.
//!
//! Run the correct version:        cargo test -p temporal
//! Run the buggy version (fails):  cargo test -p temporal --features buggy

/// A media item is just an identity (which video/image/etc.).
pub type MediaItem = u32;

/// Point-like media: a single instant `t` on some media item.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalPoint {
    pub media: MediaItem,
    pub t: i64,
}

/// Range-like media: an interval `[t0, t1]` on some media item.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalInterval {
    pub media: MediaItem,
    pub t0: i64,
    pub t1: i64,
}

/// Two intervals overlap iff they share a media item and their ranges intersect.
pub fn temporal_overlap(a: &TemporalInterval, b: &TemporalInterval) -> bool {
    a.media == b.media && a.t0 <= b.t1 && b.t0 <= a.t1
}

/// A point lies inside an interval iff they share a media item and the point's
/// instant is within `[t0, t1]`.
///
/// This is `point_in_interval`, a distinct relation from `temporal_overlap`.
#[cfg(not(feature = "buggy"))]
pub fn point_in_interval(p: &TemporalPoint, i: &TemporalInterval) -> bool {
    p.media == i.media && i.t0 <= p.t && p.t <= i.t1
}

/// The video's bug, made concrete.
///
/// To reuse the interval-overlap machinery, the point is given a phantom
/// duration `[p.t, p.t + 5]` ("a frame lasts a few units") and fed through
/// `temporal_overlap`. The diff looks reasonable and passes naive tests — but
/// it has quietly turned a point into a range. Now a point sitting just before
/// an interval is reported as inside it (whenever `p.t < i.t0 <= p.t + 5`).
/// The generated proptest, whose oracle is the *real* law, catches exactly that.
#[cfg(feature = "buggy")]
pub fn point_in_interval(p: &TemporalPoint, i: &TemporalInterval) -> bool {
    let degenerate = TemporalInterval {
        media: p.media,
        t0: p.t,
        t1: p.t + 5,
    };
    temporal_overlap(&degenerate, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_basic() {
        let a = TemporalInterval {
            media: 0,
            t0: 0,
            t1: 10,
        };
        let b = TemporalInterval {
            media: 0,
            t0: 5,
            t1: 15,
        };
        assert!(temporal_overlap(&a, &b));
        let c = TemporalInterval {
            media: 0,
            t0: 20,
            t1: 30,
        };
        assert!(!temporal_overlap(&a, &c));
    }

    #[test]
    fn point_inside_and_outside() {
        let i = TemporalInterval {
            media: 0,
            t0: 0,
            t1: 10,
        };
        assert!(point_in_interval(&TemporalPoint { media: 0, t: 5 }, &i));
        assert!(!point_in_interval(&TemporalPoint { media: 0, t: 11 }, &i));
        // different media never relate
        assert!(!point_in_interval(&TemporalPoint { media: 1, t: 5 }, &i));
    }
}
