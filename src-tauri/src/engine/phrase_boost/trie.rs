//! Token-level boosting trie, ported from ParakeetWeb's `BoostingTrie`
//! (`phraseBoost.js`). The trie itself is immutable once built; a decode
//! position is described by an *active set* (the root plus every node of a
//! phrase in progress), owned by the caller so each beam hypothesis can carry
//! its own.
//!
//! Bonus of the node at depth `d` of a phrase of `L` tokens and weight `w`:
//! `w * (1 - COMMITMENT_SCALING * (1 - d / L)) * (1 + DEPTH_SCALING * (d - 1))`.
//! Example with `w = 1` and a 4-token phrase: 0.25, 0.75, 1.5, 2.5. The first
//! token earns little (one token of four is thin evidence, and a standing
//! bonus on common first tokens inserts spurious words), a completed phrase
//! earns its full depth-scaled weight.
//!
//! `COMMITMENT_SCALING` is ParakeetWeb's default; a list can override it
//! (`#!commitment 0`: 1, 1.5, 2, 2.5). With greedy decoding a phrase whose
//! first token loses is never entered, so its deeper bonuses never fire:
//! on the UltiMed validation clips, commitment 0 with min-p 0.01 found more
//! medical terms and lowered the WER of the big French medical list.

use super::list::DEFAULT_BOOST_MIN_P;

const DEPTH_SCALING: f32 = 0.5;
const COMMITMENT_SCALING: f32 = 1.0;

pub type NodeId = u32;
pub const ROOT: NodeId = 0;

/// Token ids of one phrase spelling, with its weight and min-p gate.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedPhrase {
    pub ids: Vec<i32>,
    pub weight: f32,
    pub minp: f32,
}

#[derive(Debug, Clone)]
struct Node {
    /// Sorted by token id, for binary search.
    children: Vec<(i32, NodeId)>,
    /// Strongest boost and strongest penalty of the phrases through this
    /// node, kept apart so a user penalty is never swallowed by a larger
    /// boost on a shared prefix. The decoder reads their sum.
    pos: f32,
    neg: f32,
    /// Min-p gate of the strongest-magnitude phrase through this node.
    minp: f32,
}

impl Node {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            pos: 0.0,
            neg: 0.0,
            minp: DEFAULT_BOOST_MIN_P,
        }
    }

    fn bonus(&self) -> f32 {
        self.pos + self.neg
    }
}

#[derive(Debug, Clone)]
pub struct BoostingTrie {
    nodes: Vec<Node>,
    /// Most permissive min-p inserted: no token below `max + ln(min_minp)`
    /// can clear any gate, which lets `boosts` skip most of the vocabulary.
    min_minp: f32,
}

impl BoostingTrie {
    pub fn new(phrases: &[EncodedPhrase]) -> Self {
        Self::with_commitment(phrases, COMMITMENT_SCALING)
    }

    /// `commitment` in [0, 1]: 1 ramps the bonus with `d / L`, 0 disables the ramp.
    pub fn with_commitment(phrases: &[EncodedPhrase], commitment: f32) -> Self {
        let mut trie = Self {
            nodes: vec![Node::new()],
            min_minp: 1.0,
        };
        for phrase in phrases.iter().filter(|p| !p.ids.is_empty()) {
            trie.insert(phrase, commitment);
        }
        trie
    }

    fn insert(&mut self, phrase: &EncodedPhrase, commitment: f32) {
        self.min_minp = self.min_minp.min(phrase.minp);
        let len = phrase.ids.len() as f32;
        let mut node = ROOT;
        for (i, &id) in phrase.ids.iter().enumerate() {
            let child = match self.child(node, id) {
                Some(child) => child,
                None => self.add_child(node, id),
            };
            let depth = (i + 1) as f32;
            let committed = 1.0 - commitment * (1.0 - depth / len);
            let bonus = phrase.weight * committed * (1.0 + DEPTH_SCALING * (depth - 1.0));
            let n = &mut self.nodes[child as usize];
            let prev_mag = n.pos.max(-n.neg);
            if bonus >= 0.0 {
                n.pos = n.pos.max(bonus);
            } else {
                n.neg = n.neg.min(bonus);
            }
            if bonus.abs() > prev_mag {
                n.minp = phrase.minp;
            }
            node = child;
        }
    }

    fn add_child(&mut self, parent: NodeId, id: i32) -> NodeId {
        let child = self.nodes.len() as NodeId;
        self.nodes.push(Node::new());
        let children = &mut self.nodes[parent as usize].children;
        let at = children.partition_point(|&(t, _)| t < id);
        children.insert(at, (id, child));
        child
    }

    fn child(&self, node: NodeId, id: i32) -> Option<NodeId> {
        let children = &self.nodes[node as usize].children;
        children
            .binary_search_by_key(&id, |&(t, _)| t)
            .ok()
            .map(|i| children[i].1)
    }

    #[cfg(test)]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes[ROOT as usize].children.is_empty()
    }

    /// Active set at the start of a decode: the root alone.
    pub fn start(&self) -> Vec<NodeId> {
        vec![ROOT]
    }

    /// Strongest-magnitude `(bonus, minp)` proposed for token `id` by the
    /// active set, or `None` when no active node continues with it.
    fn boost_for(&self, active: &[NodeId], id: i32) -> Option<(f32, f32)> {
        active
            .iter()
            .filter_map(|&node| self.child(node, id))
            .map(|child| &self.nodes[child as usize])
            .max_by(|a, b| a.bonus().abs().total_cmp(&b.bonus().abs()))
            .map(|n| (n.bonus(), n.minp))
    }

    /// Sparse `(token, bonus)` list for one frame: every token the active set
    /// continues with whose probability is at least its min-p times the most
    /// likely token's (`logit >= max_logit + ln(minp)`). Boosting therefore
    /// only reorders tokens the model already finds plausible. The caller adds
    /// the bonuses to its scores; `logits` stays untouched, so confidences are
    /// computed from the model's true logits.
    pub fn boosts(&self, logits: &[f32], active: &[NodeId]) -> Vec<(usize, f32)> {
        if self.is_empty() {
            return Vec::new();
        }
        let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let floor = max_logit + self.min_minp.ln();
        logits
            .iter()
            .enumerate()
            .filter(|&(_, &l)| l >= floor)
            .filter_map(|(id, &l)| {
                let (bonus, minp) = self.boost_for(active, id as i32)?;
                (l >= max_logit + minp.ln()).then_some((id, bonus))
            })
            .collect()
    }

    /// Active set after emitting the non-blank token `id`: the root (a new
    /// phrase can always start) plus each active node's child for `id`.
    pub fn advance(&self, active: &[NodeId], id: i32) -> Vec<NodeId> {
        let mut next = vec![ROOT];
        for child in active.iter().filter_map(|&node| self.child(node, id)) {
            if !next.contains(&child) {
                next.push(child);
            }
        }
        next
    }
}

#[cfg(test)]
mod tests {
    // Cases ported from ParakeetWeb's test/unit/phrase-boost.test.mjs.
    use super::*;

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "expected ~{expected}, got {actual}"
        );
    }

    fn phrase(ids: &[i32], weight: f32) -> EncodedPhrase {
        EncodedPhrase {
            ids: ids.to_vec(),
            weight,
            minp: DEFAULT_BOOST_MIN_P,
        }
    }

    fn node_for(trie: &BoostingTrie, ids: &[i32]) -> Node {
        let mut node = ROOT;
        for &id in ids {
            node = trie.child(node, id).expect("path exists");
        }
        trie.nodes[node as usize].clone()
    }

    const SIX: [i32; 6] = [10, 11, 12, 13, 14, 15];

    #[test]
    fn bonus_ramps_with_commitment_and_depth() {
        let trie = BoostingTrie::new(&[phrase(&SIX, 2.0)]);
        let active = trie.start();
        close(trie.boost_for(&active, 10).unwrap().0, 2.0 / 6.0);
        assert_eq!(trie.boost_for(&active, 11), None);
        assert_eq!(trie.boost_for(&active, 10).unwrap().1, DEFAULT_BOOST_MIN_P);

        let active = trie.advance(&active, 10);
        close(
            trie.boost_for(&active, 11).unwrap().0,
            2.0 * (2.0 / 6.0) * 1.5,
        );
        close(trie.boost_for(&active, 10).unwrap().0, 2.0 / 6.0);

        let active = trie.advance(&active, 999);
        assert_eq!(active, vec![ROOT]);
        assert_eq!(trie.boost_for(&active, 11), None);
    }

    #[test]
    fn zero_commitment_gives_the_first_token_its_full_weight() {
        let trie = BoostingTrie::with_commitment(&[phrase(&SIX, 2.0)], 0.0);
        let active = trie.start();
        close(trie.boost_for(&active, 10).unwrap().0, 2.0);
        let active = trie.advance(&active, 10);
        close(trie.boost_for(&active, 11).unwrap().0, 2.0 * 1.5);
    }

    #[test]
    fn completed_phrase_earns_full_depth_scaled_weight() {
        let trie = BoostingTrie::new(&[phrase(&SIX, 2.0)]);
        let mut active = trie.start();
        for &id in &SIX[..5] {
            active = trie.advance(&active, id);
        }
        close(
            trie.boost_for(&active, 15).unwrap().0,
            2.0 * (1.0 + 0.5 * 5.0),
        );
        let single = BoostingTrie::new(&[phrase(&[10], 2.0)]);
        close(single.boost_for(&single.start(), 10).unwrap().0, 2.0);
    }

    #[test]
    fn longer_phrase_speculates_less_on_the_same_first_token() {
        let short = BoostingTrie::new(&[phrase(&SIX[..2], 1.0)]);
        let long = BoostingTrie::new(&[phrase(&SIX, 1.0)]);
        assert!(node_for(&long, &[10]).bonus() < node_for(&short, &[10]).bonus());
    }

    #[test]
    fn boosts_respect_the_min_p_gate() {
        let trie = BoostingTrie::new(&[EncodedPhrase {
            ids: vec![1],
            weight: 5.0,
            minp: 0.1,
        }]);
        // Token 1 is e^-5 = 0.7% as likely as token 0: below a 10% gate.
        assert!(trie.boosts(&[6.0, 1.0, 0.0], &trie.start()).is_empty());
        // On a flat frame the same logit is 37% as likely: boosted.
        assert_eq!(trie.boosts(&[2.0, 1.0, 0.0], &trie.start()), vec![(1, 5.0)]);
        let loose = BoostingTrie::new(&[EncodedPhrase {
            ids: vec![1],
            weight: 5.0,
            minp: 0.001,
        }]);
        assert_eq!(
            loose.boosts(&[6.0, 1.0, 0.0], &loose.start()),
            vec![(1, 5.0)]
        );
    }

    #[test]
    fn negative_weight_penalizes() {
        let trie = BoostingTrie::new(&[phrase(&[0], -3.0)]);
        assert_eq!(trie.boosts(&[2.0, 1.5], &trie.start()), vec![(0, -3.0)]);
    }

    #[test]
    fn penalty_is_never_swallowed_by_a_shared_prefix_boost() {
        let t = BoostingTrie::new(&[
            EncodedPhrase {
                ids: vec![1, 2, 3],
                weight: 1.5,
                minp: 0.025,
            },
            EncodedPhrase {
                ids: vec![1],
                weight: -0.25,
                minp: 0.025,
            },
        ]);
        // At depth scaling 0.5 the 3-token phrase's first node is 1.5/3 = 0.5.
        close(node_for(&t, &[1]).bonus(), 0.25);

        let a = BoostingTrie::new(&[phrase(&[1, 2], 1.5), phrase(&[1], -1.0)]);
        let b = BoostingTrie::new(&[phrase(&[1], -1.0), phrase(&[1, 2], 1.5)]);
        assert_eq!(node_for(&a, &[1]).bonus(), node_for(&b, &[1]).bonus());

        // Strongest boost and strongest penalty only, never a sum.
        let t = BoostingTrie::new(&[
            phrase(&[1, 2], 1.5),
            phrase(&[1, 3], 1.5),
            phrase(&[1], -0.5),
            phrase(&[1], -0.25),
        ]);
        close(node_for(&t, &[1]).bonus(), 0.75 - 0.5);
    }

    #[test]
    fn min_p_gate_comes_from_the_strongest_phrase() {
        let t = BoostingTrie::new(&[
            EncodedPhrase {
                ids: vec![1, 2],
                weight: 4.0,
                minp: 0.4,
            },
            EncodedPhrase {
                ids: vec![1],
                weight: -1.0,
                minp: 0.1,
            },
        ]);
        assert_eq!(node_for(&t, &[1]).minp, 0.4);
        let t = BoostingTrie::new(&[
            EncodedPhrase {
                ids: vec![1, 2],
                weight: 4.0,
                minp: 0.4,
            },
            EncodedPhrase {
                ids: vec![1],
                weight: -3.0,
                minp: 0.1,
            },
        ]);
        assert_eq!(node_for(&t, &[1]).minp, 0.1);
    }

    #[test]
    fn active_sets_track_overlapping_phrases() {
        // "1 2" and "2 3": after emitting 1 then 2, both the end of the first
        // phrase and the start of the second are active.
        let t = BoostingTrie::new(&[phrase(&[1, 2], 1.0), phrase(&[2, 3], 1.0)]);
        let a = t.advance(&t.advance(&t.start(), 1), 2);
        assert_eq!(a.len(), 3);
        assert!(t.boost_for(&a, 3).is_some());
    }

    #[test]
    fn empty_trie_boosts_nothing() {
        let t = BoostingTrie::new(&[]);
        assert!(t.is_empty());
        assert!(t.boosts(&[1.0, 2.0], &t.start()).is_empty());
    }
}
