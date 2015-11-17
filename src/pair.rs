use std::{cmp, ptr};

use self::ApplyResult::*;
use self::Leading::*;

#[allow(non_camel_case_types)]
pub type blk = u64;
pub const BCNT: u8 = 5;
pub const BLK_BITS: u8 = 64;

pub const BCNT_: usize = BCNT as usize;
pub const VAL_BITS: u8 = BLK_BITS - 8;
pub const VAL_MASK: blk = (1 << VAL_BITS) - 1;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Leading {
    Top,
    Bot,
}

impl Leading {
    pub fn switched(self) -> Leading {
        match self {
            Top => Bot,
            Bot => Top,
        }
    }
}

#[derive(Copy, Clone)]
pub struct SPart(blk);

#[derive(Copy, Clone)]
pub struct SPair {
    a: SPart,
    b: SPart,
}

/// A variable length pair.
#[derive(Copy, Clone)]
pub struct VPair {
    data: [blk; BCNT_],
    /// Who is leading.
    leading: Leading,
    /// How many bits of the first block are shared.
    prefix: u8,
    /// How many bits of the last block are used.
    used: u8,
    /// which block is last used one
    tail: u8,
}

#[derive(Copy, Clone)]
struct VHead {
    data: blk,
    /// How many bits are shared
    prefix: u8,
    /// How many bits are used
    used: u8,
}

impl VPair {
    pub fn new() -> VPair {
        VPair {
            data: [0; BCNT_],
            prefix: 0,
            used: 0,
            leading: Top,
            tail: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.tail == 0 && self.used == 0
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
                    MatchRemaining => unreachable!("SPairs contain at most VAL_BITS bit, which always fit in BITS bit"),
                }
            }
        }
    }

    pub fn leading(&self) -> Leading {
        self.leading
    }

    pub fn len(&self) -> u32 {
        if self.tail == 0 {
            (self.used - self.prefix) as u32
        } else {
            (self.tail as u32 - 1) * BLK_BITS as u32 + (BLK_BITS - self.prefix) as u32 + self.used as u32
        }
    }

    fn with_offset_prefix_and_add_lead(&self, offset: u8, prefix: u8, lead: SPart) -> VPair {
        let mut p = VPair {
            data: [0; BCNT_],
            prefix: prefix,
            used: self.used,
            leading: self.leading,
            tail: self.tail - offset
        };

        unsafe {
            ptr::copy_nonoverlapping(&self.data[offset as usize], &mut p.data[0], BCNT_ - offset as usize)

        }

        p.apply_lead(lead);

        p
    }

    fn switched_with_new_lead(&self, lead: SPart) -> VPair {
        let mut p = VPair {
            data: [0; BCNT_],
            prefix: 0,
            used: lead.len(),
            leading: self.leading.switched(),
            tail: 0,
        };

        p.data[0] = lead.data();

        p
    }

    fn apply_lead(&mut self, mut lead: SPart) {
        let pushable;

        { // make the borrowchecker happy
            let last = &mut self.data[self.tail as usize];

            // push all we can into the current byte
            pushable = cmp::min(BLK_BITS - self.used, lead.len());
            *last |= lead.shift_data(pushable) << self.used;
        }

        let new_used = self.used + pushable;

        // if new_used is 64 need to open a new block
        if lead.len() == 0 && new_used < BLK_BITS {
            self.used = new_used;
        } else {
            self.push(lead.data());
            self.used = lead.len();
        }

        debug_assert!(self.used != BLK_BITS);
    }

    fn push(&mut self, block: blk) {
        assert!(self.tail + 1 < BCNT, "lead too long, increase BCNT");
        self.tail += 1;
        self.data[self.tail as usize] = block;
    }

    fn head(&self) -> VHead {
        debug_assert!(self.used != BLK_BITS);

        VHead {
            data: self.data[0],
            prefix: self.prefix,
            used: if self.tail > 0 { BLK_BITS } else { self.used },
        }
    }

    fn head2(&self) -> VHead {
        debug_assert!(self.tail > 0);

        VHead {
            data: self.data[1],
            prefix: 0,
            used: if self.tail > 1 { BLK_BITS } else { self. used },
        }
    }
}

#[derive(Copy, Clone)]
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

impl SPair {
    pub fn new(a: SPart, b: SPart) -> SPair {
        SPair {
            a: a,
            b: b,
        }
    }
}

impl SPart {
    pub fn new(len: blk, val: blk) -> SPart {
        assert!(len <= VAL_BITS as u64);
        assert!((val & VAL_MASK) == val);

        SPart((len << VAL_BITS) | val)
    }

    fn data(&self) -> blk {
        self.0 & VAL_MASK
    }

    fn len(&self) -> u8 {
        (self.0 >> VAL_BITS) as u8
    }

    fn shift(&mut self, shift: u8) {
        let new_len = self.len() - shift;
        let new_val = self.data() >> shift;
        self.0 = ((new_len as blk) << VAL_BITS) | new_val;
    }

    fn shift_data(&mut self, shift: u8) -> blk {
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

        if self.prefix == BLK_BITS {
            debug_assert!(self.used == BLK_BITS);
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
fn mask(cnt: u8) -> blk {
    (1 << cnt) - 1
}
