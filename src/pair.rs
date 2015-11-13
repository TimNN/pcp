use std::cmp;

use self::ApplyResult::*;
use super::Leading::{self, Top, Bot};

#[derive(Copy, Clone)]
struct SPart(u64);

#[derive(Copy, Clone)]
pub struct SPair {
    a: SPart,
    b: SPart,
}

/// A variable length pair.
#[derive(Clone)]
pub struct VPair {
    data: Vec<u64>,
    /// Who is leading.
    leading: Leading,
    /// How many bits of the first block are shared.
    prefix: u8,
    /// How many bits of the last block are used.
    used: u8,
}

#[derive(Copy, Clone)]
struct VHead {
    data: u64,
    /// How many bits are shared
    prefix: u8,
    /// How many bits are used
    used: u8,
}

impl VPair {
    pub fn new() -> VPair {
        let mut v = Vec::with_capacity(1);
        v.push(0);

        VPair {
            data: v,
            prefix: 0,
            used: 0,
            leading: Top,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.data.len() == 1 && self.used == 0
    }

    pub fn apply(&self, p: &SPair) -> Option<VPair> {
        let (mut lead, mut follow) = match self.leading {
            Top => (p.a, p.b),
            Bot => (p.b, p.a),
        };

        let mut head = self.head();

        match head.apply(&mut follow, &mut lead) {
            Mismatch => None,
            Match => Some(self.with_offset_prefix_and_add_lead(0, head.prefix, lead)),
            LeadSwitch => Some(self.switched_with_new_lead(follow)),
            MatchRemaining => {
                let mut head2 = self.head2();

                match head2.apply(&mut follow, &mut lead) {
                    Mismatch => None,
                    Match => Some(self.with_offset_prefix_and_add_lead(1, head2.prefix, lead)),
                    LeadSwitch => Some(self.switched_with_new_lead(follow)),
                    MatchRemaining => unreachable!("SPairs contain at most 56 bit, which always fit in 64 bit"),
                }
            }
        }
    }

    fn with_offset_prefix_and_add_lead(&self, offset: usize, prefix: u8, lead: SPart) -> VPair {
        // Reserve an additional block in case apply_lead spills over
        let mut v = Vec::with_capacity(self.data.len() + 1 - offset);
        v.push_all(&self.data[offset..]);

        let mut p = VPair {
            data: v,
            prefix: prefix,
            used: self.used,
            leading: self.leading,
        };

        p.apply_lead(lead);

        p
    }

    fn switched_with_new_lead(&self, lead: SPart) -> VPair {
        let mut v = Vec::with_capacity(1);
        v.push(lead.data());

        VPair {
            data: v,
            prefix: 0,
            used: lead.len(),
            leading: self.leading.switched(),
        }
    }

    fn apply_lead(&mut self, mut lead: SPart) {
        let pushable;

        { // make the borrowchecker happy
            let last = self.data.last_mut().unwrap();

            // push all we can into the current byte
            pushable = cmp::min(64 - self.used, lead.len());
            *last |= lead.shift_data(pushable) << self.used;
        }

        let new_used = self.used + pushable;

        // if new_used is 64 need to open a new block
        if lead.len() == 0 && new_used < 64 {
            self.used = new_used;
        } else {
            self.data.push(lead.data());
            self.used = lead.len();
        }

        debug_assert!(self.used != 64);
    }

    fn head(&self) -> VHead {
        debug_assert!(self.used != 64);

        VHead {
            data: self.data[0],
            prefix: self.prefix,
            used: if self.data.len() > 1 { 64 } else { self.used },
        }
    }

    fn head2(&self) -> VHead {
        debug_assert!(self.data.len() > 1);

        VHead {
            data: self.data[1],
            prefix: 0,
            used: if self.data.len() > 2 { 64 } else { self. used },
        }
    }
}

enum ApplyResult {
    /// No match
    Mismatch,
    /// Completely matched, bits in lead need to be appended
    Match,
    /// Matched but end of block reached, repeat with second block
    MatchRemaining,
    /// Completely caught up, bits in follow make up a new block
    LeadSwitch,
}

impl SPart {
    fn data(&self) -> u64 {
        self.0 & 0x00FF_FFFF_FFFF_FFFF
    }

    fn len(&self) -> u8 {
        (self.0 >> 56) as u8
    }

    fn shift(&mut self, shift: u8) {
        let new_len = self.len() - shift;
        let new_val = self.data() >> shift;
        self.0 = ((new_len as u64) << 56) | new_val;
    }

    fn shift_data(&mut self, shift: u8) -> u64 {
        let data = self.0 & mask(shift);
        self.shift(shift);
        data
    }

    fn shift_prefix(a: &mut SPart, b: &mut SPart) {
        // after the xor the first mismatched bit is a one
        // theres no need for masking data since we need to use the min of the
        // lens and prefix anyway
        let prefix = (a.0 ^ b.0).trailing_zeros() as u8; // 0-64 always fit in an u8
        let prefix = cmp::min(prefix, cmp::min(a.len(), b.len()));
        a.shift(prefix);
        b.shift(prefix);
    }
}

impl VHead {
    fn apply(&mut self, follow: &mut SPart, lead: &mut SPart) -> ApplyResult {
        // fist, check that overlapping follows are equal
        let overlap = cmp::min(self.used - self.prefix, follow.len());

        // check
        let m = mask(overlap);
        let h = (self.data >> self.prefix) & m;
        let p = follow.data() & m;

        if h != p { return Mismatch; }

        // update
        self.prefix += overlap;
        follow.shift(overlap);

        if self.prefix == 64 {
            debug_assert!(self.used == 64);
            return MatchRemaining; // Reached the end of block, needs to be removed
        }

        // at this point follow has caught up to lead, so we can eliminate any common prefix from follow and lead
        SPart::shift_prefix(follow, lead);

        match (follow.len() == 0, lead.len() == 0) {
            (_, true) => LeadSwitch, // lead is empty, start a new block with follow
            (false, false) => Mismatch, // both have non matching bits remaining
            (true, false) => Match, // follow is empty
        }
    }
}

/// Create a bitmask of which the lower `cnt` bits are set.
fn mask(cnt: u8) -> u64 {
    (1 << cnt) - 1
}

impl<'a> From<&'a str> for SPart {
    fn from(s: &'a str) -> SPart {
        let mut val = 0u64;
        let mut len = 0u64;

        for b in s.bytes().rev() {
            len += 1;
            val <<= 1;

            match b {
                b'0' => (),
                b'1' => val |= 1,
                _ => panic!("invalid input byte: {}", b),
            }
        }

        assert!(len <= 56, "invalid input: too long");

        SPart((len << 56) | val)
    }
}

impl<'a> From<(&'a str, &'a str)> for SPair {
    fn from((a, b): (&'a str, &'a str)) -> SPair {
        SPair {
            a: a.into(),
            b: b.into(),
        }
    }
}
