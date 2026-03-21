/// Trie node for ignore.
#[derive(Default, Debug)]
pub struct TrieNode {
    #[cfg(feature = "hashbrown")]
    /// Children for trie.
    pub children: hashbrown::HashMap<u8, TrieNode>,
    #[cfg(not(feature = "hashbrown"))]
    /// Children for trie.
    pub children: std::collections::HashMap<u8, TrieNode>,
    /// End of word match.
    pub is_end_of_word: bool,
}

/// Basic Ignore trie.
#[derive(Debug)]
pub struct Trie {
    /// The trie node.
    pub root: TrieNode,
}

impl Trie {
    /// Setup a new trie.
    pub fn new() -> Self {
        Trie {
            root: TrieNode::default(),
        }
    }
    // Insert a word into the Trie.
    pub fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for &b in word.as_bytes() {
            node = node.children.entry(b).or_default();
        }
        node.is_end_of_word = true;
    }

    // Check if the Trie contains any prefix of the given string.
    #[inline]
    pub fn contains_prefix(&self, text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut node = &self.root;

        for &b in bytes {
            if let Some(next_node) = node.children.get(&b) {
                node = next_node;
                if node.is_end_of_word {
                    return true;
                }
            } else {
                break;
            }
        }

        false
    }
}
