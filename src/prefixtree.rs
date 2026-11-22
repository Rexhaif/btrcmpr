// Each new suffix becomes its bucket's root. Repartition the old search path
// around it; untouched subtrees retain their lexicographic order.
pub(crate) struct Tree {
    roots: Vec<u32>,
    links: Vec<u32>,
}
impl Tree {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            roots: vec![0; 1 << 17],
            links: vec![0; 2 * n],
        }
    }
    #[inline]
    pub(crate) fn insert<const DEPTH: usize>(&mut self, s: &[u8], p: usize) -> [u32; 2] {
        let w = crate::lz::word(s, p);
        let key = ((w << 40).wrapping_mul(0x9e3779b97f4a7c15) >> 47) as usize;
        let mut cur = std::mem::replace(&mut self.roots[key], p as u32 + 1);
        let mut left = 2 * p;
        let mut right = left + 1;
        let (mut lower, mut upper) = (0usize, 0usize);
        let (mut best, mut out) = (2, [u32::MAX; 2]);
        let cap = (s.len() - p).min(64);
        for _ in 0..DEPTH {
            if cur == 0 {
                break;
            }
            let q = (cur - 1) as usize;
            let mut l = lower.min(upper);
            while l + 8 <= cap {
                let diff = crate::lz::word(s, p + l) ^ crate::lz::word(s, q + l);
                l += diff.trailing_zeros() as usize / 8;
                if diff != 0 {
                    break;
                }
            }
            while l < cap && s[p + l] == s[q + l] {
                l += 1;
            }
            if l > best {
                out = [cur - 1, out[0]];
                best = l;
            }
            if l == cap {
                self.links[left] = self.links[2 * q];
                self.links[right] = self.links[2 * q + 1];
                return out;
            }
            if s[q + l] < s[p + l] {
                self.links[left] = cur;
                left = 2 * q + 1;
                cur = self.links[left];
                lower = l;
            } else {
                self.links[right] = cur;
                right = 2 * q;
                cur = self.links[right];
                upper = l;
            }
        }
        self.links[left] = 0;
        self.links[right] = 0;
        out
    }
}
