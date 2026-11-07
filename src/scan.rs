//! Byte scanning: the inner loops of the tokenizer (text up to the next `<`,
//! quoted attribute values up to the closing quote, comments up to `-->`, …)
//! and of entity decoding (text up to the next `&`).
//!
//! Every search goes through [`find_byte`], so the optional `simd` feature can
//! swap the portable loop for `memchr` in one place. Both implementations
//! return the index of the *first* match, so they produce identical tokens; a
//! unit test (compiled with the feature on) checks them against each other.

/// The index of the first `needle` byte in `haystack`, if any.
#[inline]
pub(crate) fn find_byte(needle: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(feature = "simd")]
    {
        memchr::memchr(needle, haystack)
    }
    #[cfg(not(feature = "simd"))]
    {
        portable(needle, haystack)
    }
}

/// [`find_byte`], and whether an `&` comes before the match (anywhere, if
/// there is none): the tokenizer notes it while it scans text and attribute
/// values, so that decoding them can skip its own search for an `&`. `needle`
/// must not be `&`.
#[inline]
pub(crate) fn find_byte_noting_amp(needle: u8, haystack: &[u8]) -> (Option<usize>, bool) {
    #[cfg(feature = "simd")]
    {
        match memchr::memchr2(needle, b'&', haystack) {
            None => (None, false),
            Some(i) if haystack.get(i) == Some(&needle) => (Some(i), false),
            Some(i) => match haystack.get(i + 1..) {
                Some(rest) => (memchr::memchr(needle, rest).map(|j| i + 1 + j), true),
                None => (None, true),
            },
        }
    }
    #[cfg(not(feature = "simd"))]
    {
        portable_noting_amp(needle, haystack)
    }
}

/// The index of the first occurrence of `needle` in `haystack`, if any:
/// [`find_byte`] for its first byte, then a check of the rest. An empty
/// `needle` matches at 0.
#[inline]
pub(crate) fn find(needle: &[u8], haystack: &[u8]) -> Option<usize> {
    let Some((&first, rest)) = needle.split_first() else {
        return Some(0);
    };
    let mut from = 0;
    loop {
        let at = from + find_byte(first, haystack.get(from..)?)?;
        if haystack
            .get(at + 1..)
            .is_some_and(|after| after.starts_with(rest))
        {
            return Some(at);
        }
        from = at + 1;
    }
}

/// The dependency-free search: "SIMD within a register", 16 bytes per step
/// as two `u64` words (the same trick as `core`'s private `memchr`), then a
/// byte loop for the tail. On a long run with no match (Apple M4) it scans
/// ~37 GB/s against ~4 GB/s for an iterator `position` loop, which the
/// compiler does not vectorize because of the early exit; `memchr`'s NEON
/// code does ~110 GB/s.
#[cfg(any(not(feature = "simd"), test))]
fn portable(needle: u8, haystack: &[u8]) -> Option<usize> {
    const LO: u64 = 0x0101_0101_0101_0101;
    const HI: u64 = 0x8080_8080_8080_8080;
    // A word whose bytes are zero exactly where `w` holds `needle`; the
    // returned mask flags them. Borrows only ever run towards higher bytes,
    // so in little-endian order the lowest flag is the first match.
    let splat = LO * u64::from(needle);
    let flags = |w: u64| {
        let x = w ^ splat;
        x.wrapping_sub(LO) & !x & HI
    };
    // (Always `Some` on a 16-byte chunk; the fallback matches nothing.)
    let word = |b: Option<&[u8; 8]>| b.map_or(splat ^ HI, |b| u64::from_le_bytes(*b));
    let (chunks, tail) = haystack.as_chunks::<16>();
    let mut offset = 0;
    for chunk in chunks {
        let (a, b) = (word(chunk.first_chunk()), word(chunk.last_chunk()));
        let (fa, fb) = (flags(a), flags(b));
        if fa | fb != 0 {
            let bit = if fa != 0 {
                fa.trailing_zeros()
            } else {
                64 + fb.trailing_zeros()
            };
            return Some(offset + bit as usize / 8);
        }
        offset += 16;
    }
    tail.iter().position(|&b| b == needle).map(|i| offset + i)
}

/// [`portable`] that also reports whether an `&` comes before the match
/// ([`find_byte_noting_amp`]): the same two-word steps with a second set of
/// flags. A flag is only ever wrong above a true match of its byte (the
/// borrow runs towards higher bytes), so a flag below the first `needle`
/// is a real `&`.
#[cfg(any(not(feature = "simd"), test))]
fn portable_noting_amp(needle: u8, haystack: &[u8]) -> (Option<usize>, bool) {
    const LO: u64 = 0x0101_0101_0101_0101;
    const HI: u64 = 0x8080_8080_8080_8080;
    let splat = LO * u64::from(needle);
    let amp = LO * u64::from(b'&');
    let flags = |w: u64, splat: u64| {
        let x = w ^ splat;
        x.wrapping_sub(LO) & !x & HI
    };
    let word = |b: Option<&[u8; 8]>| b.map_or(splat ^ HI, |b| u64::from_le_bytes(*b));
    let (chunks, tail) = haystack.as_chunks::<16>();
    let mut offset = 0;
    let mut seen = false;
    for chunk in chunks {
        let (a, b) = (word(chunk.first_chunk()), word(chunk.last_chunk()));
        let (fa, fb) = (flags(a, splat), flags(b, splat));
        let (ga, gb) = (flags(a, amp), flags(b, amp));
        if fa | fb != 0 {
            let bit = if fa != 0 {
                fa.trailing_zeros()
            } else {
                64 + fb.trailing_zeros()
            };
            // The `&` flags of the bytes before the match's.
            let before = if bit < 64 {
                ga & ((1u64 << bit) - 1)
            } else {
                ga | (gb & ((1u64 << (bit - 64)) - 1))
            };
            return (Some(offset + bit as usize / 8), seen || before != 0);
        }
        seen |= (ga | gb) != 0;
        offset += 16;
    }
    for (i, &b) in tail.iter().enumerate() {
        if b == needle {
            return (Some(offset + i), seen);
        }
        seen |= b == b'&';
    }
    (None, seen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// Haystacks around the sizes where SIMD searches switch strategy (short
    /// tails, one vector, several vectors, unaligned starts).
    fn haystacks() -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for len in [
            0usize, 1, 2, 7, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 257, 1000,
        ] {
            for at in [None, Some(0), Some(len / 2), Some(len.saturating_sub(1))] {
                let mut h: Vec<u8> = (0..len).map(|i| b'a' + (i % 23) as u8).collect();
                if let Some(at) = at {
                    if let Some(b) = h.get_mut(at) {
                        *b = b'<';
                    }
                    // A second match later on: the first must win.
                    if let Some(b) = h.get_mut(at + 9) {
                        *b = b'<';
                    }
                }
                out.push(h);
            }
        }
        out
    }

    /// The obviously correct search, as the reference.
    fn naive(needle: u8, haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|&b| b == needle)
    }

    #[test]
    fn the_portable_search_finds_the_first_match() {
        for h in haystacks() {
            for start in 0..h.len().min(40) {
                let h = &h[start..];
                for needle in [b'<', b'a', b'b', b'&', 0, 0x80, 0xff] {
                    assert_eq!(portable(needle, h), naive(needle, h), "{needle} in {h:?}");
                }
            }
        }
        // Every byte value, in every lane, next to every other byte value
        // (borrow propagation between lanes is where SWAR searches go wrong).
        for needle in 0..=255u8 {
            for other in 0..=255u8 {
                for lane in 0..17 {
                    let mut h = [other; 17];
                    h[lane] = needle;
                    assert_eq!(portable(needle, &h), naive(needle, &h));
                }
            }
        }
    }

    /// With the `simd` feature, `find_byte` is `memchr`: it must agree with
    /// the default build's search everywhere, or the two builds would
    /// tokenize differently.
    #[cfg(feature = "simd")]
    #[test]
    fn memchr_and_the_portable_search_agree() {
        for h in haystacks() {
            for start in 0..h.len().min(40) {
                let h = &h[start..];
                for needle in [b'<', b'a', b'&', b'"', b'\'', 0, 0xff] {
                    assert_eq!(memchr::memchr(needle, h), portable(needle, h));
                    assert_eq!(find_byte(needle, h), portable(needle, h));
                }
            }
        }
        for needle in 0..=255u8 {
            for other in 0..=255u8 {
                let mut h = [other; 40];
                h[33] = needle;
                assert_eq!(find_byte(needle, &h), portable(needle, &h));
            }
        }
    }

    /// The obviously correct version of [`find_byte_noting_amp`].
    fn naive_amp(needle: u8, haystack: &[u8]) -> (Option<usize>, bool) {
        let at = naive(needle, haystack);
        let before = &haystack[..at.unwrap_or(haystack.len())];
        (at, before.contains(&b'&'))
    }

    #[test]
    fn noting_the_amp_agrees_with_the_naive_search() {
        for h in haystacks() {
            for start in 0..h.len().min(40) {
                let h = &h[start..];
                for needle in [b'<', b'"', b'a', 0, 0xff] {
                    assert_eq!(portable_noting_amp(needle, h), naive_amp(needle, h));
                    assert_eq!(find_byte_noting_amp(needle, h), naive_amp(needle, h));
                    // The same with an `&` put at each of a few places.
                    for at in [0, 1, 7, 8, 15, 16, 17, 31, 40] {
                        let mut h = h.to_vec();
                        if let Some(b) = h.get_mut(at) {
                            *b = b'&';
                        }
                        assert_eq!(find_byte_noting_amp(needle, &h), naive_amp(needle, &h));
                    }
                }
            }
        }
        // Every byte value around a match and an `&`, in every pair of lanes
        // (borrows between lanes are where SWAR searches go wrong).
        for needle in *b"<\"'>" {
            for other in 0..=255u8 {
                for lane in 0..33 {
                    for amp_lane in 0..33 {
                        if lane == amp_lane {
                            continue;
                        }
                        let mut h = [other; 33];
                        h[lane] = needle;
                        h[amp_lane] = b'&';
                        assert_eq!(find_byte_noting_amp(needle, &h), naive_amp(needle, &h));
                    }
                }
            }
        }
    }

    #[test]
    fn find_finds_the_first_full_match() {
        assert_eq!(find(b"-->", b"a - -- --> -->"), Some(7));
        assert_eq!(find(b"-->", b"--"), None);
        assert_eq!(find(b"-->", b"-->"), Some(0));
        assert_eq!(find(b"-->", b"--->"), Some(1));
        assert_eq!(find(b"]]>", b"]]]>"), Some(1));
        assert_eq!(find(b"", b"x"), Some(0));
        assert_eq!(find(b"x", b""), None);
    }
}
