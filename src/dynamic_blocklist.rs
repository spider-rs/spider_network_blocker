use crate::trie::Trie;
use arc_swap::ArcSwap;
use std::sync::Arc;

/// Immutable snapshot of trie layers. Each `extend` adds a layer
/// instead of cloning and rebuilding.
#[derive(Default)]
struct Layers {
    tries: Vec<Arc<Trie>>,
}

impl Layers {
    fn with_trie(trie: Trie) -> Self {
        Self {
            tries: vec![Arc::new(trie)],
        }
    }

    #[inline]
    fn contains_prefix(&self, text: &str) -> bool {
        for trie in &self.tries {
            if trie.contains_prefix(text) {
                return true;
            }
        }
        false
    }

    fn len(&self) -> usize {
        self.tries.len()
    }
}

/// A lock-free, dynamically updatable block list backed by layered [`Trie`]s.
///
/// Reads are wait-free (atomic pointer load). `extend` builds a small trie
/// from only the new patterns and appends it as a layer — no cloning of
/// existing data. `compact` merges all layers into one when you want to
/// reclaim the overhead of multiple layers.
pub struct DynamicBlockList {
    layers: ArcSwap<Layers>,
}

impl DynamicBlockList {
    /// Create an empty dynamic block list.
    pub fn new() -> Self {
        Self {
            layers: ArcSwap::from_pointee(Layers::default()),
        }
    }

    /// Create a dynamic block list pre-seeded with `patterns`.
    pub fn from_patterns<'a>(patterns: impl IntoIterator<Item = &'a str>) -> Self {
        let mut trie = Trie::new();
        for p in patterns {
            trie.insert(p);
        }
        Self {
            layers: ArcSwap::from_pointee(Layers::with_trie(trie)),
        }
    }

    /// Lock-free check — returns `true` if `url` matches any prefix in any
    /// layer of the current snapshot.
    #[inline]
    pub fn is_blocked(&self, url: &str) -> bool {
        self.layers.load().contains_prefix(url)
    }

    /// Atomically replace all layers with a single `new_trie`.
    ///
    /// In-flight readers keep using the old snapshot; new readers see
    /// `new_trie` immediately.
    pub fn swap(&self, new_trie: Trie) {
        self.layers
            .store(Arc::new(Layers::with_trie(new_trie)));
    }

    /// Replace all layers with a single trie built from `patterns`.
    ///
    /// Builds the trie first, then swaps — the hot path is never blocked.
    pub fn seed<'a>(&self, patterns: impl IntoIterator<Item = &'a str>) {
        let mut trie = Trie::new();
        for p in patterns {
            trie.insert(p);
        }
        self.layers
            .store(Arc::new(Layers::with_trie(trie)));
    }

    /// Extend the block list with additional patterns — lock-free, no cloning.
    ///
    /// Builds a small trie from only the new patterns and appends it as a new
    /// layer. Existing layers are shared via `Arc` — zero copying.
    /// Concurrent `extend` calls may race (last writer wins on the layer list)
    /// which is fine for additive seeding.
    pub fn extend<'a>(&self, patterns: impl IntoIterator<Item = &'a str>) {
        let mut trie = Trie::new();
        for p in patterns {
            trie.insert(p);
        }
        let current = self.layers.load();
        let mut new_layers = Layers {
            tries: current.tries.clone(), // clones Arc pointers, not trie data
        };
        new_layers.tries.push(Arc::new(trie));
        self.layers.store(Arc::new(new_layers));
    }

    /// Merge all layers into a single trie to reclaim per-layer lookup overhead.
    ///
    /// Call this periodically after many `extend` calls. Reads remain lock-free
    /// throughout — the compacted trie is swapped in atomically.
    pub fn compact(&self) {
        let current = self.layers.load();
        if current.len() <= 1 {
            return;
        }
        let mut merged = Trie::new();
        for trie in &current.tries {
            collect_into(trie, &mut merged);
        }
        self.layers
            .store(Arc::new(Layers::with_trie(merged)));
    }

    /// Number of trie layers. Useful for deciding when to `compact`.
    pub fn layer_count(&self) -> usize {
        self.layers.load().len()
    }
}

impl Default for DynamicBlockList {
    fn default() -> Self {
        Self::new()
    }
}

/// Walk `source` and re-insert every stored word into `dest`.
fn collect_into(source: &Trie, dest: &mut Trie) {
    let mut stack: Vec<(&crate::trie::TrieNode, Vec<u8>)> = vec![(&source.root, Vec::new())];

    while let Some((node, prefix)) = stack.pop() {
        if node.is_end_of_word {
            if let Ok(word) = std::str::from_utf8(&prefix) {
                dest.insert(word);
            }
        }
        for (&byte, child) in &node.children {
            let mut next_prefix = prefix.clone();
            next_prefix.push(byte);
            stack.push((child, next_prefix));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_blocks_nothing() {
        let bl = DynamicBlockList::new();
        assert!(!bl.is_blocked("https://example.com"));
    }

    #[test]
    fn test_from_patterns() {
        let bl = DynamicBlockList::from_patterns([
            "https://ads.example.com/",
            "https://tracker.example.com/",
        ]);
        assert!(bl.is_blocked("https://ads.example.com/banner.js"));
        assert!(bl.is_blocked("https://tracker.example.com/pixel"));
        assert!(!bl.is_blocked("https://cdn.example.com/app.js"));
    }

    #[test]
    fn test_seed_replaces() {
        let bl = DynamicBlockList::from_patterns(["https://old.example.com/"]);
        assert!(bl.is_blocked("https://old.example.com/x"));

        bl.seed(["https://new.example.com/"]);
        assert!(!bl.is_blocked("https://old.example.com/x"));
        assert!(bl.is_blocked("https://new.example.com/y"));
    }

    #[test]
    fn test_extend_adds_without_cloning_tries() {
        let bl = DynamicBlockList::from_patterns(["https://ads.example.com/"]);
        assert_eq!(bl.layer_count(), 1);

        bl.extend(["https://tracker.example.com/"]);
        assert_eq!(bl.layer_count(), 2);

        // Both layers are checked
        assert!(bl.is_blocked("https://ads.example.com/banner.js"));
        assert!(bl.is_blocked("https://tracker.example.com/pixel"));
        assert!(!bl.is_blocked("https://cdn.example.com/app.js"));
    }

    #[test]
    fn test_multiple_extends() {
        let bl = DynamicBlockList::from_patterns(["https://ads.example.com/"]);
        bl.extend(["https://tracker.example.com/"]);
        bl.extend(["https://analytics.example.com/"]);
        bl.extend(["https://pixel.example.com/"]);

        assert_eq!(bl.layer_count(), 4);
        assert!(bl.is_blocked("https://ads.example.com/x"));
        assert!(bl.is_blocked("https://tracker.example.com/x"));
        assert!(bl.is_blocked("https://analytics.example.com/x"));
        assert!(bl.is_blocked("https://pixel.example.com/x"));
    }

    #[test]
    fn test_compact_merges_layers() {
        let bl = DynamicBlockList::from_patterns(["https://ads.example.com/"]);
        bl.extend(["https://tracker.example.com/"]);
        bl.extend(["https://analytics.example.com/"]);
        assert_eq!(bl.layer_count(), 3);

        bl.compact();
        assert_eq!(bl.layer_count(), 1);

        // All patterns still present after compaction
        assert!(bl.is_blocked("https://ads.example.com/x"));
        assert!(bl.is_blocked("https://tracker.example.com/x"));
        assert!(bl.is_blocked("https://analytics.example.com/x"));
        assert!(!bl.is_blocked("https://cdn.example.com/x"));
    }

    #[test]
    fn test_compact_noop_single_layer() {
        let bl = DynamicBlockList::from_patterns(["https://ads.example.com/"]);
        bl.compact(); // should be a no-op
        assert_eq!(bl.layer_count(), 1);
        assert!(bl.is_blocked("https://ads.example.com/x"));
    }

    #[test]
    fn test_swap() {
        let bl = DynamicBlockList::from_patterns(["https://old.example.com/"]);
        let mut new_trie = Trie::new();
        new_trie.insert("https://new.example.com/");
        bl.swap(new_trie);

        assert!(!bl.is_blocked("https://old.example.com/x"));
        assert!(bl.is_blocked("https://new.example.com/y"));
    }

    #[test]
    fn test_concurrent_reads_during_extend() {
        use std::sync::Arc;

        let bl = Arc::new(DynamicBlockList::from_patterns([
            "https://ads.example.com/",
        ]));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let bl = Arc::clone(&bl);
                std::thread::spawn(move || bl.is_blocked("https://ads.example.com/banner.js"))
            })
            .collect();

        // Extend while reads are in flight
        bl.extend(["https://new.example.com/"]);

        for h in handles {
            let _ = h.join().unwrap();
        }

        // After extend, both old and new patterns are visible
        assert!(bl.is_blocked("https://ads.example.com/x"));
        assert!(bl.is_blocked("https://new.example.com/x"));
    }
}
