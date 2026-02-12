use spider_network_blocker::adblock::ADBLOCK_PATTERNS;
use spider_network_blocker::intercept_manager::NetworkInterceptManager;
use spider_network_blocker::scripts::{
    URL_IGNORE_EMBEDED_TRIE, URL_IGNORE_SCRIPT_BASE_PATHS, URL_IGNORE_SCRIPT_STYLES_PATHS,
    URL_IGNORE_TRIE, URL_IGNORE_TRIE_PATHS,
};
use spider_network_blocker::xhr::{URL_IGNORE_XHR_MEDIA_TRIE, URL_IGNORE_XHR_TRIE};

fn create_url(url: &str) -> Option<Box<url::Url>> {
    url::Url::parse(url).ok().map(Box::new)
}

// ── End-to-end script blocking ────────────────────────────────────────

#[test]
fn test_e2e_script_blocking_google_analytics() {
    assert!(URL_IGNORE_TRIE.contains_prefix("https://www.google-analytics.com/analytics.js"));
    assert!(URL_IGNORE_TRIE.contains_prefix("https://www.googletagmanager.com/gtm.js?id=GTM-XXXX"));
    assert!(URL_IGNORE_TRIE.contains_prefix("https://www.googleanalytics.com/ga.js"));
}

#[test]
fn test_e2e_script_blocking_allows_legitimate() {
    // Stripe should NOT be blocked
    assert!(!URL_IGNORE_TRIE.contains_prefix("https://js.stripe.com/v3/"));
    // Google Fonts should NOT be blocked
    assert!(!URL_IGNORE_TRIE.contains_prefix("https://fonts.googleapis.com/css2?family=Roboto"));
    // CDN content should NOT be blocked
    assert!(!URL_IGNORE_TRIE.contains_prefix("https://cdn.jsdelivr.net/npm/vue@3/dist/vue.global.js"));
}

#[test]
fn test_e2e_script_blocking_facebook_pixel() {
    assert!(URL_IGNORE_TRIE.contains_prefix("https://connect.facebook.net/en_US/fbevents.js"));
    assert!(URL_IGNORE_TRIE.contains_prefix("https://connect.facebook.net/signals/config/12345"));
}

#[test]
fn test_e2e_script_blocking_hotjar() {
    assert!(URL_IGNORE_TRIE.contains_prefix("https://static.hotjar.com/c/hotjar-12345.js"));
    assert!(URL_IGNORE_TRIE.contains_prefix("https://script.hotjar.com/modules.abc123.js"));
}

// ── End-to-end XHR blocking ──────────────────────────────────────────

#[test]
fn test_e2e_xhr_blocking() {
    assert!(URL_IGNORE_XHR_TRIE.contains_prefix("https://googleads.g.doubleclick.net/pagead/id"));
    assert!(URL_IGNORE_XHR_TRIE.contains_prefix("https://analytics.google.com/g/collect?v=2"));
    assert!(URL_IGNORE_XHR_TRIE.contains_prefix("https://sentry.io/api/12345/envelope/"));
    assert!(URL_IGNORE_XHR_TRIE.contains_prefix("https://www.clarity.ms/collect"));
}

#[test]
fn test_e2e_xhr_allows_legitimate() {
    assert!(!URL_IGNORE_XHR_TRIE.contains_prefix("https://api.stripe.com/v1/charges"));
    assert!(!URL_IGNORE_XHR_TRIE.contains_prefix("https://api.example.com/v1/users"));
}

#[test]
fn test_e2e_xhr_media_trie() {
    assert!(URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://www.youtube.com/s/player/abc"));
    assert!(URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://api.spotify.com/v1/tracks/xyz"));
    assert!(URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://maps.googleapis.com/maps/api/js"));
    assert!(!URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://api.example.com/data"));
}

// ── Domain-specific manager routing ──────────────────────────────────

#[test]
fn test_manager_routing_known_domains() {
    let cases = vec![
        ("https://www.amazon.com/dp/B08N5WRWNW", NetworkInterceptManager::Amazon),
        ("https://www.reddit.com/r/rust", NetworkInterceptManager::Reddit),
        ("https://www.nytimes.com/section/world", NetworkInterceptManager::Nytimes),
        ("https://www.facebook.com/user", NetworkInterceptManager::Facebook),
    ];

    for (url, expected) in cases {
        let parsed = create_url(url);
        let mgr = NetworkInterceptManager::new(&parsed);
        assert_eq!(mgr, expected, "Domain routing failed for {}", url);
    }
}

#[test]
fn test_manager_routing_unknown_domain() {
    let parsed = create_url("https://www.totally-unknown-site.com/page");
    let mgr = NetworkInterceptManager::new(&parsed);
    assert_eq!(mgr, NetworkInterceptManager::Unknown);
}

#[test]
fn test_manager_intercept_detection_unknown() {
    let parsed = create_url("https://www.unknown.com/page");
    let mgr = NetworkInterceptManager::new(&parsed);
    // Unknown manager should not block anything via domain-specific rules
    assert!(!mgr.intercept_detection("https://cdn.example.com/app.js", false, false));
}

// ── Cross-trie consistency ───────────────────────────────────────────

#[test]
fn test_cross_trie_google_analytics_blocked_in_scripts() {
    assert!(URL_IGNORE_TRIE.contains_prefix("https://www.google-analytics.com/analytics.js"));
    // Should also appear as generic pattern
    assert!(URL_IGNORE_TRIE.contains_prefix("analytics.js"));
}

#[test]
fn test_cross_trie_embedded_vs_script() {
    // YouTube embeds should be in embedded trie
    assert!(URL_IGNORE_EMBEDED_TRIE.contains_prefix("https://www.youtube.com/embed/abc123"));
    // GoogleTagManager should be in both embedded and script tries
    assert!(URL_IGNORE_EMBEDED_TRIE.contains_prefix("https://www.googletagmanager.com/gtm.js"));
    assert!(URL_IGNORE_TRIE.contains_prefix("https://www.googletagmanager.com/gtm.js"));
}

#[test]
fn test_suffix_pattern_matching() {
    // Patterns like ".onetrust.com" should match when they appear as prefix of input
    assert!(URL_IGNORE_TRIE.contains_prefix(".onetrust.com/consent/12345"));
    assert!(URL_IGNORE_TRIE.contains_prefix(".newrelic.com/nr-123.js"));
    assert!(URL_IGNORE_TRIE.contains_prefix("doubleclick.net/ads/pixel"));
}

// ── Path-based blocking ──────────────────────────────────────────────

#[test]
fn test_wp_plugin_blocking() {
    assert!(URL_IGNORE_SCRIPT_BASE_PATHS.contains_prefix("wp-content/plugins/cookie-law-info/frontend.js"));
    assert!(URL_IGNORE_SCRIPT_BASE_PATHS.contains_prefix("wp-content/plugins/borlabs-cookie/cookie.js"));
    assert!(!URL_IGNORE_SCRIPT_BASE_PATHS.contains_prefix("wp-content/plugins/woocommerce/assets/main.js"));
}

#[test]
fn test_wp_theme_and_style_blocking() {
    assert!(URL_IGNORE_SCRIPT_STYLES_PATHS.contains_prefix("wp-content/themes/twentytwentyfour/style.css"));
    assert!(URL_IGNORE_SCRIPT_STYLES_PATHS.contains_prefix("wp-content/plugins/contact-form-7/frontend.js"));
    assert!(!URL_IGNORE_SCRIPT_STYLES_PATHS.contains_prefix("wp-content/plugins/woocommerce/main.js"));
}

#[test]
fn test_trie_paths_standalone() {
    assert!(URL_IGNORE_TRIE_PATHS.contains_prefix("tracking.js"));
    assert!(URL_IGNORE_TRIE_PATHS.contains_prefix("analytics.js"));
    assert!(URL_IGNORE_TRIE_PATHS.contains_prefix("ads.js"));
    assert!(URL_IGNORE_TRIE_PATHS.contains_prefix("tracking.min.js"));
    assert!(URL_IGNORE_TRIE_PATHS.contains_prefix("analytics.min.js"));
    assert!(URL_IGNORE_TRIE_PATHS.contains_prefix("_vercel/insights/script.js"));
    assert!(!URL_IGNORE_TRIE_PATHS.contains_prefix("main.js"));
    assert!(!URL_IGNORE_TRIE_PATHS.contains_prefix("app.bundle.js"));
}

// ── ADBLOCK_PATTERNS integrity ───────────────────────────────────────

#[test]
fn test_adblock_patterns_not_empty() {
    assert!(!ADBLOCK_PATTERNS.is_empty(), "ADBLOCK_PATTERNS should not be empty");
    assert!(ADBLOCK_PATTERNS.len() > 30, "Expected 30+ patterns, got {}", ADBLOCK_PATTERNS.len());
}

#[test]
fn test_adblock_patterns_contain_expected() {
    let patterns = &*ADBLOCK_PATTERNS;
    assert!(patterns.contains(&"googletagmanager.com"));
    assert!(patterns.contains(&"googlesyndication.com"));
    assert!(patterns.contains(&"analytics.js"));
    assert!(patterns.contains(&"tracking.js"));
    assert!(patterns.contains(&"ads.js"));
    assert!(patterns.contains(&"-advertisement."));
    assert!(patterns.contains(&"g.doubleclick.net"));
}

#[test]
fn test_adblock_patterns_no_empty_strings() {
    for pattern in ADBLOCK_PATTERNS.iter() {
        assert!(!pattern.is_empty(), "ADBLOCK_PATTERNS should not contain empty strings");
        assert!(pattern.len() > 1, "Pattern too short: {}", pattern);
    }
}

// ── Adblock engine tests (feature-gated) ─────────────────────────────

#[cfg(feature = "adblock")]
mod adblock_engine_tests {
    use spider_network_blocker::adblock::engine::{AdblockEngine, FilterListUrls};

    #[test]
    fn test_engine_creation_from_rules() {
        let rules = vec![
            "||googletagmanager.com^",
            "||google-analytics.com^",
            "||doubleclick.net^",
        ];
        let engine = AdblockEngine::from_rules(rules, false);

        assert!(engine.should_block(
            "https://www.googletagmanager.com/gtm.js",
            "https://example.com",
            "script"
        ));
        assert!(engine.should_block(
            "https://www.google-analytics.com/analytics.js",
            "https://example.com",
            "script"
        ));
        assert!(!engine.should_block(
            "https://cdn.example.com/app.js",
            "https://example.com",
            "script"
        ));
    }

    #[test]
    fn test_engine_blocking_xhr() {
        let rules = vec![
            "||doubleclick.net^",
            "||analytics.google.com^",
        ];
        let engine = AdblockEngine::from_rules(rules, false);

        assert!(engine.should_block(
            "https://googleads.g.doubleclick.net/pagead/id",
            "https://example.com",
            "xmlhttprequest"
        ));
    }

    #[test]
    fn test_engine_allows_legitimate_urls() {
        let rules = vec![
            "||googletagmanager.com^",
            "||doubleclick.net^",
        ];
        let engine = AdblockEngine::from_rules(rules, false);

        assert!(!engine.should_block(
            "https://js.stripe.com/v3/",
            "https://example.com",
            "script"
        ));
        assert!(!engine.should_block(
            "https://fonts.googleapis.com/css2?family=Roboto",
            "https://example.com",
            "stylesheet"
        ));
    }

    #[test]
    fn test_engine_serialization_roundtrip() {
        let rules = vec![
            "||googletagmanager.com^",
            "||doubleclick.net^",
            "||hotjar.com^",
        ];
        let engine = AdblockEngine::from_rules(rules, false);

        let serialized = engine.serialize();
        assert!(!serialized.is_empty(), "Serialized data should not be empty");

        let restored = AdblockEngine::deserialize(&serialized)
            .expect("Deserialization should succeed");

        // Verify the restored engine works identically
        assert!(restored.should_block(
            "https://www.googletagmanager.com/gtm.js",
            "https://example.com",
            "script"
        ));
        assert!(restored.should_block(
            "https://static.hotjar.com/c/hotjar-12345.js",
            "https://example.com",
            "script"
        ));
        assert!(!restored.should_block(
            "https://cdn.example.com/app.js",
            "https://example.com",
            "script"
        ));
    }

    #[test]
    fn test_engine_into_shared_thread_safety() {
        let rules = vec![
            "||googletagmanager.com^",
            "||doubleclick.net^",
        ];
        let engine = AdblockEngine::from_rules(rules, false);
        let shared = engine.into_shared();

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let engine_clone = shared.clone();
                std::thread::spawn(move || {
                    engine_clone.should_block(
                        "https://www.googletagmanager.com/gtm.js",
                        "https://example.com",
                        "script",
                    )
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.join().unwrap(), "All threads should detect the block");
        }
    }

    #[test]
    fn test_engine_from_filter_list_content() {
        let content = "[Adblock Plus 2.0]\n\
                        ! Title: Test List\n\
                        ! Last modified: 2024-01-01\n\
                        ||googletagmanager.com^\n\
                        ||hotjar.com^\n\
                        ||facebook.net/en_US/fbevents.js\n";

        let engine = AdblockEngine::from_filter_list_content(content, false);

        assert!(engine.should_block(
            "https://www.googletagmanager.com/gtm.js",
            "https://example.com",
            "script"
        ));
        assert!(engine.should_block(
            "https://static.hotjar.com/c/hotjar-12345.js",
            "https://example.com",
            "script"
        ));
        assert!(!engine.should_block(
            "https://cdn.example.com/app.js",
            "https://example.com",
            "script"
        ));
    }

    #[test]
    fn test_engine_check_request_returns_result() {
        let rules = vec!["||googletagmanager.com^"];
        let engine = AdblockEngine::from_rules(rules, false);

        let result = engine.check_request(
            "https://www.googletagmanager.com/gtm.js",
            "https://example.com",
            "script",
        );
        assert!(result.is_some());
        assert!(result.unwrap().matched);

        let result = engine.check_request(
            "https://cdn.example.com/app.js",
            "https://example.com",
            "script",
        );
        assert!(result.is_some());
        assert!(!result.unwrap().matched);
    }

    #[test]
    fn test_filter_list_urls_constants() {
        assert!(FilterListUrls::EASYLIST.starts_with("https://"));
        assert!(FilterListUrls::EASYLIST.contains("easylist"));
        assert!(FilterListUrls::EASYPRIVACY.starts_with("https://"));
        assert!(FilterListUrls::EASYPRIVACY.contains("easyprivacy"));
    }
}
