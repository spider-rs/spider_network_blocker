#![recursion_limit = "256"]
/// Scripts to block;
pub mod scripts;
/// Trie tree.
pub mod trie;
/// Adblock patterns.
pub mod adblock;
/// Xhr block patterns.
pub mod xhr;
/// interception manager
pub mod intercept_manager;