use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spider_network_blocker::intercept_manager::NetworkInterceptManager;
use spider_network_blocker::scripts::{
    URL_IGNORE_EMBEDED_TRIE, URL_IGNORE_SCRIPT_BASE_PATHS, URL_IGNORE_TRIE,
};
use spider_network_blocker::xhr::{URL_IGNORE_XHR_MEDIA_TRIE, URL_IGNORE_XHR_TRIE};

fn bench_trie_prefix_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_prefix_matching");

    // --- Script trie hits ---
    let script_hit_urls = [
        "https://www.google-analytics.com/analytics.js",
        "https://www.googletagmanager.com/gtm.js?id=GTM-XXXXX",
        "https://connect.facebook.net/en_US/fbevents.js",
        "https://static.hotjar.com/c/hotjar-12345.js",
        "https://cdn.segment.com/analytics.js/v1/abc/analytics.min.js",
        "https://static.cloudflareinsights.com/beacon.min.js",
    ];

    group.bench_function("script_trie_hits", |b| {
        b.iter(|| {
            for url in &script_hit_urls {
                black_box(URL_IGNORE_TRIE.contains_prefix(url));
            }
        })
    });

    // --- Script trie misses ---
    let script_miss_urls = [
        "https://cdn.example.com/app.bundle.js",
        "https://api.stripe.com/v1/payment_intents",
        "https://fonts.googleapis.com/css2?family=Roboto",
        "https://cdn.jsdelivr.net/npm/vue@3/dist/vue.global.js",
        "https://unpkg.com/react@18/umd/react.production.min.js",
        "https://example.com/assets/main.css",
    ];

    group.bench_function("script_trie_misses", |b| {
        b.iter(|| {
            for url in &script_miss_urls {
                black_box(URL_IGNORE_TRIE.contains_prefix(url));
            }
        })
    });

    // --- XHR trie hits ---
    let xhr_hit_urls = [
        "https://googleads.g.doubleclick.net/pagead/id",
        "https://analytics.google.com/g/collect?v=2&tid=G-XXX",
        "https://sentry.io/api/12345/envelope/",
        "https://www.clarity.ms/collect",
        "https://static.hotjar.com/c/hotjar-12345.js",
    ];

    group.bench_function("xhr_trie_hits", |b| {
        b.iter(|| {
            for url in &xhr_hit_urls {
                black_box(URL_IGNORE_XHR_TRIE.contains_prefix(url));
            }
        })
    });

    // --- XHR trie misses ---
    let xhr_miss_urls = [
        "https://api.example.com/v1/users",
        "https://cdn.shopify.com/s/files/1/product.jpg",
        "https://api.stripe.com/v1/charges",
        "https://graphql.example.com/query",
    ];

    group.bench_function("xhr_trie_misses", |b| {
        b.iter(|| {
            for url in &xhr_miss_urls {
                black_box(URL_IGNORE_XHR_TRIE.contains_prefix(url));
            }
        })
    });

    // --- Embedded trie ---
    group.bench_function("embedded_trie_hits", |b| {
        b.iter(|| {
            black_box(URL_IGNORE_EMBEDED_TRIE.contains_prefix("https://www.youtube.com/embed/abc123"));
            black_box(URL_IGNORE_EMBEDED_TRIE.contains_prefix("https://player.vimeo.com/video/12345"));
            black_box(URL_IGNORE_EMBEDED_TRIE.contains_prefix("https://www.google.com/maps/embed?pb=!1m14"));
        })
    });

    // --- XHR media trie ---
    group.bench_function("xhr_media_trie_hits", |b| {
        b.iter(|| {
            black_box(URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://www.youtube.com/s/player/abc"));
            black_box(URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://api.spotify.com/v1/tracks"));
            black_box(URL_IGNORE_XHR_MEDIA_TRIE.contains_prefix("https://maps.googleapis.com/maps/api"));
        })
    });

    // --- Base path trie ---
    group.bench_function("base_path_trie", |b| {
        b.iter(|| {
            black_box(URL_IGNORE_SCRIPT_BASE_PATHS.contains_prefix("wp-content/plugins/cookie-law-info/frontend.js"));
            black_box(URL_IGNORE_SCRIPT_BASE_PATHS.contains_prefix("analytics/track.js"));
            black_box(URL_IGNORE_SCRIPT_BASE_PATHS.contains_prefix("assets/main.js"));
        })
    });

    group.finish();
}

fn bench_intercept_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("intercept_detection");

    // Pre-create managers for known domains
    let amazon_url = url::Url::parse("https://www.amazon.com/dp/B08N5WRWNW").ok().map(Box::new);
    let reddit_url = url::Url::parse("https://www.reddit.com/r/rust").ok().map(Box::new);
    let nytimes_url = url::Url::parse("https://www.nytimes.com/section/world").ok().map(Box::new);
    let unknown_url = url::Url::parse("https://www.example.com/page").ok().map(Box::new);

    let amazon_mgr = NetworkInterceptManager::new(&amazon_url);
    let reddit_mgr = NetworkInterceptManager::new(&reddit_url);
    let nytimes_mgr = NetworkInterceptManager::new(&nytimes_url);
    let unknown_mgr = NetworkInterceptManager::new(&unknown_url);

    group.bench_function("amazon_script_intercept", |b| {
        b.iter(|| {
            black_box(amazon_mgr.intercept_detection(
                "https://www.google-analytics.com/analytics.js",
                false,
                false,
            ));
        })
    });

    group.bench_function("reddit_xhr_intercept", |b| {
        b.iter(|| {
            black_box(reddit_mgr.intercept_detection(
                "https://www.redditstatic.com/ads/pixel.js",
                false,
                true,
            ));
        })
    });

    group.bench_function("nytimes_script_intercept", |b| {
        b.iter(|| {
            black_box(nytimes_mgr.intercept_detection(
                "https://static.hotjar.com/c/hotjar-12345.js",
                false,
                false,
            ));
        })
    });

    group.bench_function("unknown_domain_fallback", |b| {
        b.iter(|| {
            black_box(unknown_mgr.intercept_detection(
                "https://cdn.example.com/app.js",
                false,
                false,
            ));
        })
    });

    group.bench_function("manager_creation", |b| {
        b.iter(|| {
            let url = url::Url::parse("https://www.amazon.com/dp/B08N5WRWNW")
                .ok()
                .map(Box::new);
            black_box(NetworkInterceptManager::new(&url));
        })
    });

    group.finish();
}

#[cfg(feature = "adblock")]
fn bench_adblock_engine(c: &mut Criterion) {
    use spider_network_blocker::adblock::engine::AdblockEngine;

    let mut group = c.benchmark_group("adblock_engine");

    let rules = vec![
        "||googletagmanager.com^",
        "||google-analytics.com^",
        "||doubleclick.net^",
        "||facebook.net/en_US/fbevents.js",
        "||hotjar.com^",
        "||segment.com/analytics.js",
        "||sentry.io^$third-party",
    ];

    group.bench_function("engine_creation", |b| {
        b.iter(|| {
            black_box(AdblockEngine::from_rules(rules.clone(), false));
        })
    });

    let engine = AdblockEngine::from_rules(rules, false);

    let hit_urls = [
        ("https://www.googletagmanager.com/gtm.js", "https://example.com", "script"),
        ("https://www.google-analytics.com/analytics.js", "https://example.com", "script"),
        ("https://googleads.g.doubleclick.net/pagead/id", "https://example.com", "script"),
    ];

    group.bench_function("engine_check_hits", |b| {
        b.iter(|| {
            for (url, source, req_type) in &hit_urls {
                black_box(engine.should_block(url, source, req_type));
            }
        })
    });

    let miss_urls = [
        ("https://cdn.example.com/app.js", "https://example.com", "script"),
        ("https://api.stripe.com/v1/charges", "https://example.com", "xhr"),
        ("https://fonts.googleapis.com/css2", "https://example.com", "stylesheet"),
    ];

    group.bench_function("engine_check_misses", |b| {
        b.iter(|| {
            for (url, source, req_type) in &miss_urls {
                black_box(engine.should_block(url, source, req_type));
            }
        })
    });

    group.finish();
}

#[cfg(not(feature = "adblock"))]
criterion_group!(benches, bench_trie_prefix_matching, bench_intercept_detection);

#[cfg(feature = "adblock")]
criterion_group!(
    benches,
    bench_trie_prefix_matching,
    bench_intercept_detection,
    bench_adblock_engine
);

criterion_main!(benches);
