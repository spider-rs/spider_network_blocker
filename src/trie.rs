/// Trie node for ignore.
#[derive(Default, Debug)]
pub struct TrieNode {
    #[cfg(feature = "hashbrown")]
    /// Children for trie.
    pub children: hashbrown::HashMap<char, TrieNode>,
    #[cfg(not(feature = "hashbrown"))]
    /// Children for trie.
    pub children: std::hash::HashMap<char, TrieNode>,
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
        for ch in word.chars() {
            node = node.children.entry(ch).or_insert_with(TrieNode::default);
        }
        node.is_end_of_word = true;
    }

    // Check if the Trie contains any prefix of the given string.
    #[inline]
    pub fn contains_prefix(&self, text: &str) -> bool {
        let mut node = &self.root;

        for ch in text.chars() {
            if let Some(next_node) = node.children.get(&ch) {
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
