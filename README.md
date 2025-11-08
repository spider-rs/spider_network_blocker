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
let scipt_blocked = URL_IGNORE_TRIE.contains_prefix(".doubleclick.net");
```

## Contributing

Contributions and improvements are welcome. Feel free to open issues or submit pull requests on the GitHub repository.

## License

This project is licensed under the MIT License.