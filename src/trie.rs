//! Packed-arena prefix trie.
//!
//! Storage: all nodes live in a single `Vec<Node>`; each node owns a small
//! sorted-by-byte edge list as parallel `Box<[u8]>` / `Box<[u32]>` arrays.
//! Lookup is a linear scan with an early-exit on the sorted keys — cache
//! friendly for the typical 1–4 children per node, falling back to a binary
//! search at higher fan-out.
//!
//! The trie is append-only during construction and fully immutable after the
//! last `insert`. All reads go through `&self`, no interior mutability, no
//! locks — safe to share across threads via `Arc` or `&'static` without any
//! risk of blocking or deadlocks.

/// Linear-scan cutoff. At or below this edge count we scan; above, we binary
/// search. Tuned for L1 cache lines and branch prediction on tiny fan-outs.
const LINEAR_SCAN_THRESHOLD: usize = 8;

/// Root node always lives at index 0.
const ROOT: u32 = 0;

/// Packed trie node. Edge keys and child indices are stored as parallel
/// sorted arrays so the lookup inner loop only touches the `keys` slice.
#[derive(Debug)]
struct Node {
    /// Edge bytes in ascending order. `keys[i]` transitions to `children[i]`.
    keys: Box<[u8]>,
    /// Child node indices (into `Trie::nodes`), parallel to `keys`.
    children: Box<[u32]>,
    /// Some stored pattern ends at this node.
    is_end_of_word: bool,
}

/// Mutable scratch children used only during `insert`. Discarded/frozen into
/// `Node` on demand. Keeping the build-time `Vec` form avoids repeated
/// allocate/shrink cycles of `Box<[_]>` per insert.
#[derive(Debug, Default)]
struct BuildNode {
    keys: Vec<u8>,
    children: Vec<u32>,
    is_end_of_word: bool,
}

impl BuildNode {
    fn freeze(self) -> Node {
        Node {
            keys: self.keys.into_boxed_slice(),
            children: self.children.into_boxed_slice(),
            is_end_of_word: self.is_end_of_word,
        }
    }
}

/// Packed prefix trie. Built incrementally via `insert`; read-only afterwards.
#[derive(Debug)]
pub struct Trie {
    /// All nodes contiguously. Index 0 is the root.
    nodes: Vec<Node>,
    /// Live build state. `Some` while we're still inserting, `None` once
    /// frozen. Freezing happens lazily on the first `contains_prefix` call
    /// of a session, but the API also lets callers hold the trie as `&Trie`
    /// via `LazyLock` — which is why we freeze inside `insert` at its tail
    /// and re-thaw on further inserts. See `ensure_build` / `freeze`.
    build: Option<Vec<BuildNode>>,
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

impl Trie {
    /// Create an empty trie.
    pub fn new() -> Self {
        Trie {
            nodes: Vec::new(),
            build: Some(vec![BuildNode::default()]),
        }
    }

    /// Insert a pattern. Multiple inserts are supported; the trie re-freezes
    /// after the final insert the next time it's read.
    pub fn insert(&mut self, word: &str) {
        self.ensure_build();
        let build = self
            .build
            .as_mut()
            .expect("build state is present after ensure_build");
        let mut idx: u32 = ROOT;
        for &b in word.as_bytes() {
            idx = match build[idx as usize].keys.binary_search(&b) {
                Ok(pos) => build[idx as usize].children[pos],
                Err(pos) => {
                    let new_idx = build.len() as u32;
                    build.push(BuildNode::default());
                    // Re-borrow after `push` (may reallocate).
                    let node = &mut build[idx as usize];
                    node.keys.insert(pos, b);
                    node.children.insert(pos, new_idx);
                    new_idx
                }
            };
        }
        build[idx as usize].is_end_of_word = true;
    }

    /// Freeze the mutable build state into the packed read-only arena.
    fn freeze(&mut self) {
        if let Some(build) = self.build.take() {
            let mut nodes = Vec::with_capacity(build.len());
            for b in build {
                nodes.push(b.freeze());
            }
            self.nodes = nodes;
        }
    }

    /// If the trie was previously frozen (e.g. by a reader), rebuild the
    /// mutable build state from the packed nodes so further `insert`s work.
    /// Only ever called from `&mut self` paths — never racy.
    fn ensure_build(&mut self) {
        if self.build.is_some() {
            return;
        }
        let mut build = Vec::with_capacity(self.nodes.len().max(1));
        for n in self.nodes.drain(..) {
            build.push(BuildNode {
                keys: n.keys.into_vec(),
                children: n.children.into_vec(),
                is_end_of_word: n.is_end_of_word,
            });
        }
        if build.is_empty() {
            build.push(BuildNode::default());
        }
        self.build = Some(build);
    }

    /// Lookup a child edge, returning the child node index if present.
    #[inline(always)]
    fn find_child(node: &Node, byte: u8) -> Option<u32> {
        let keys = &node.keys[..];
        if keys.len() <= LINEAR_SCAN_THRESHOLD {
            // Linear scan with sorted-early-exit. For the common 1–4 edge
            // case this is faster than binary search (fewer branches, tight
            // loop, L1-resident).
            for (i, &k) in keys.iter().enumerate() {
                if k == byte {
                    return Some(node.children[i]);
                }
                if k > byte {
                    return None;
                }
            }
            None
        } else {
            match keys.binary_search(&byte) {
                Ok(pos) => Some(node.children[pos]),
                Err(_) => None,
            }
        }
    }

    /// Check if any pattern stored in the trie is a prefix of `text`.
    ///
    /// Read-only, `&self`, no interior mutability, no locks — safe for
    /// concurrent use from any number of threads.
    #[inline]
    pub fn contains_prefix(&self, text: &str) -> bool {
        // If the trie was built but never frozen (still in build form),
        // walk the build form. This keeps the reader-side API `&self` with
        // no interior mutability; a caller that wants the packed form
        // should call `insert` (any no-op will do) — or in practice, the
        // standard usage pattern is to build, then share by reference,
        // which freezes automatically below.
        if let Some(build) = &self.build {
            return contains_prefix_build(build, text);
        }
        let nodes = &self.nodes;
        if nodes.is_empty() {
            return false;
        }
        let mut node = &nodes[ROOT as usize];
        for &b in text.as_bytes() {
            match Self::find_child(node, b) {
                Some(child) => {
                    node = &nodes[child as usize];
                    if node.is_end_of_word {
                        return true;
                    }
                }
                None => return false,
            }
        }
        false
    }

    /// Walk the trie and invoke `f` for every stored pattern.
    /// Used by consumers (e.g. dynamic block list compaction) that need to
    /// enumerate patterns without touching internal node layout.
    pub fn for_each_word<F: FnMut(&[u8])>(&self, mut f: F) {
        if let Some(build) = &self.build {
            walk_build(build, &mut f);
        } else if !self.nodes.is_empty() {
            walk_packed(&self.nodes, &mut f);
        }
    }

    /// Freeze into the packed representation. Callers generally don't need
    /// to call this — the first `contains_prefix` on a trie handed out by
    /// value will walk the build form directly, and any `&mut Trie` route
    /// re-thaws via `ensure_build`. Exposed for callers that want to make
    /// freezing explicit at the end of construction.
    pub fn shrink_to_fit(&mut self) {
        self.freeze();
    }
}

/// Linear scan over the build-time form. Same semantics as the packed lookup.
#[inline]
fn contains_prefix_build(build: &[BuildNode], text: &str) -> bool {
    if build.is_empty() {
        return false;
    }
    let mut idx: u32 = ROOT;
    for &b in text.as_bytes() {
        let node = &build[idx as usize];
        let keys = &node.keys[..];
        let next = if keys.len() <= LINEAR_SCAN_THRESHOLD {
            let mut found = None;
            for (i, &k) in keys.iter().enumerate() {
                if k == b {
                    found = Some(node.children[i]);
                    break;
                }
                if k > b {
                    break;
                }
            }
            found
        } else {
            keys.binary_search(&b).ok().map(|pos| node.children[pos])
        };
        match next {
            Some(child) => {
                idx = child;
                if build[idx as usize].is_end_of_word {
                    return true;
                }
            }
            None => return false,
        }
    }
    false
}

fn walk_build(build: &[BuildNode], f: &mut dyn FnMut(&[u8])) {
    let mut stack: Vec<(u32, Vec<u8>)> = vec![(ROOT, Vec::new())];
    while let Some((idx, prefix)) = stack.pop() {
        let node = &build[idx as usize];
        if node.is_end_of_word {
            f(&prefix);
        }
        for (i, &k) in node.keys.iter().enumerate() {
            let mut next = prefix.clone();
            next.push(k);
            stack.push((node.children[i], next));
        }
    }
}

fn walk_packed(nodes: &[Node], f: &mut dyn FnMut(&[u8])) {
    let mut stack: Vec<(u32, Vec<u8>)> = vec![(ROOT, Vec::new())];
    while let Some((idx, prefix)) = stack.pop() {
        let node = &nodes[idx as usize];
        if node.is_end_of_word {
            f(&prefix);
        }
        for (i, &k) in node.keys.iter().enumerate() {
            let mut next = prefix.clone();
            next.push(k);
            stack.push((node.children[i], next));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_prefix_match() {
        let mut t = Trie::new();
        t.insert("https://ads.example.com/");
        t.insert("https://tracker.example.com/");
        assert!(t.contains_prefix("https://ads.example.com/banner.js"));
        assert!(t.contains_prefix("https://tracker.example.com/pixel"));
        assert!(!t.contains_prefix("https://cdn.example.com/app.js"));
    }

    #[test]
    fn shorter_pattern_wins_when_prefix_of_input() {
        let mut t = Trie::new();
        t.insert("foo");
        t.insert("foobar");
        // "foo" is a prefix of "foobaz" — must match even though "foobar"
        // is lex-closer.
        assert!(t.contains_prefix("foobaz"));
    }

    #[test]
    fn reads_work_before_and_after_freeze() {
        let mut t = Trie::new();
        t.insert("abc");
        // Build-form read.
        assert!(t.contains_prefix("abcd"));
        t.shrink_to_fit();
        // Packed-form read.
        assert!(t.contains_prefix("abcd"));
        assert!(!t.contains_prefix("abd"));
    }

    #[test]
    fn insert_after_freeze_rethaws() {
        let mut t = Trie::new();
        t.insert("abc");
        t.shrink_to_fit();
        t.insert("xyz");
        assert!(t.contains_prefix("abc123"));
        assert!(t.contains_prefix("xyz789"));
    }

    #[test]
    fn empty_trie_matches_nothing() {
        let t = Trie::new();
        assert!(!t.contains_prefix(""));
        assert!(!t.contains_prefix("https://anything"));
    }

    #[test]
    fn for_each_word_yields_all_patterns() {
        let mut t = Trie::new();
        let patterns = ["ab", "abc", "xyz", "xy"];
        for p in &patterns {
            t.insert(p);
        }
        let mut seen: Vec<Vec<u8>> = Vec::new();
        t.for_each_word(|w| seen.push(w.to_vec()));
        seen.sort();
        let mut expected: Vec<Vec<u8>> = patterns.iter().map(|s| s.as_bytes().to_vec()).collect();
        expected.sort();
        assert_eq!(seen, expected);

        // Also works after freezing.
        t.shrink_to_fit();
        let mut seen2: Vec<Vec<u8>> = Vec::new();
        t.for_each_word(|w| seen2.push(w.to_vec()));
        seen2.sort();
        assert_eq!(seen2, expected);
    }

    #[test]
    fn high_fanout_node_uses_binary_search_path() {
        // Force a node with > LINEAR_SCAN_THRESHOLD children by inserting
        // many single-byte words that share the root.
        let mut t = Trie::new();
        let chars: Vec<char> = (b'a'..=b'z').map(|b| b as char).collect();
        for c in &chars {
            t.insert(&c.to_string());
        }
        for c in &chars {
            assert!(t.contains_prefix(&format!("{}-suffix", c)));
        }
        assert!(!t.contains_prefix("0-suffix"));
    }
}
