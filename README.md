# spider_network_blocker

A Rust library to block ads, trackers, and embeds for networking.

## Installation

Add `spider_network_blocker` to your Cargo project with:

```sh
cargo add spider_network_blocker
```

```rust
use spider_network_blocker::{xhr::URL_IGNORE_XHR_TRIE, scripts::URL_IGNORE_TRIE};

let xhr_blocked = URL_IGNORE_XHR_TRIE.contains_prefix(".doubleclick.net");
let script_blocked = URL_IGNORE_TRIE.contains_prefix(".doubleclick.net");
```

## Dynamic Block List

Extend your block list at runtime without rebuilding. The `DynamicBlockList` is fully lock-free — reads are wait-free and writes never block readers.

```rust
use spider_network_blocker::dynamic_blocklist::DynamicBlockList;

// Start empty or pre-seeded
let blocklist = DynamicBlockList::new();

// Seed from a remote source (replaces all patterns)
let remote_patterns = vec![
    "https://ads.example.com/",
    "https://tracker.example.com/",
];
blocklist.seed(remote_patterns.into_iter());

// Extend with additional patterns — no cloning, just appends a new layer
blocklist.extend(["https://pixel.example.com/"]);
blocklist.extend(["https://analytics.example.com/"]);

// Lock-free check on the hot path
if blocklist.is_blocked("https://ads.example.com/banner.js") {
    // blocked
}

// After many extends, compact layers into one for lookup efficiency
blocklist.compact();
```

### Sharing across threads

```rust
use spider_network_blocker::dynamic_blocklist::DynamicBlockList;
use std::sync::Arc;

let blocklist = Arc::new(DynamicBlockList::from_patterns([
    "https://ads.example.com/",
]));

// Reader threads — wait-free, no contention
let bl = Arc::clone(&blocklist);
std::thread::spawn(move || {
    bl.is_blocked("https://ads.example.com/banner.js");
});

// Writer thread — builds new layer, atomically swaps
let bl = Arc::clone(&blocklist);
std::thread::spawn(move || {
    bl.extend(["https://new-tracker.example.com/"]);
});
```

## Contributing

Contributions and improvements are welcome. Feel free to open issues or submit pull requests on the GitHub repository.

## License

This project is licensed under the MIT License.