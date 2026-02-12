use std::sync::LazyLock;

pub static ADBLOCK_PATTERNS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        // Advertisement patterns
        "-advertisement.",
        "-advertisement-icon.",
        "-advertisement-management/",
        "-advertisement/script.",
        "-ads.",
        "-ads/script.",
        "-ad.",
        "ads.js",
        "gtm.js?",
        "googletagmanager.com",
        "ssl.google-analytics.com",
        // Tracking patterns
        "-tracking.",
        "-tracking/script.",
        ".tracking",
        ".snowplowanalytics.snowplow",
        ".mountain.com",
        "tracking.js",
        "track.js",
        "/upi/jslogger",
        "otBannerSdk.js",
        // Analytics scripts
        "analytics.js",
        "analytics.min.js",
        "ob.cityrobotflower.com",
        "siteintercept.qualtrics.com",
        "iesnare.com",
        "iovation.com",
        "googletagmanager.com",
        "forter.com",
        "/first.iovation.com",
        // Specific ad and tracking domains
        "googlesyndication.com",
        ".googlesyndication.com/safeframe/",
        "adsafeprotected.com",
        "cxense.com/",
        ".sharethis.com",
        "amazon-adsystem.com",
        "g.doubleclick.net",
        // Explicit ignore for common scripts
        "privacy-notice.js",
        "insight.min.js",
    ]
});

#[cfg(feature = "adblock")]
pub mod engine {
    use std::sync::Arc;

    /// Well-known filter list URLs for callers to fetch externally.
    pub struct FilterListUrls;

    impl FilterListUrls {
        /// EasyList — primary ad-blocking filter list.
        pub const EASYLIST: &'static str =
            "https://easylist.to/easylist/easylist.txt";
        /// EasyPrivacy — privacy/tracking filter list.
        pub const EASYPRIVACY: &'static str =
            "https://easylist.to/easylist/easyprivacy.txt";
    }

    /// Wrapper around Brave's `adblock::Engine` with an ergonomic API.
    ///
    /// The engine is `Send + Sync` because we exclude the `single-thread`
    /// default feature of the `adblock` crate.
    pub struct AdblockEngine {
        inner: adblock::Engine,
    }

    impl AdblockEngine {
        /// Build an engine from raw ABP/uBO filter rules.
        pub fn from_rules<I, S>(rules: I, debug: bool) -> Self
        where
            I: IntoIterator<Item = S>,
            S: AsRef<str>,
        {
            let mut filter_set = adblock::lists::FilterSet::new(debug);
            filter_set.add_filters(rules, adblock::lists::ParseOptions::default());
            let engine = adblock::Engine::from_filter_set(filter_set, true);
            Self { inner: engine }
        }

        /// Build an engine from the text content of an ABP filter list
        /// (e.g. EasyList). Callers are responsible for fetching the content.
        pub fn from_filter_list_content(content: &str, debug: bool) -> Self {
            let rules: Vec<&str> = content.lines().collect();
            Self::from_rules(rules, debug)
        }

        /// Check whether `url` should be blocked.
        ///
        /// - `source_url`: the page URL that initiated the request.
        /// - `request_type`: resource type (`"script"`, `"xhr"`, `"image"`, etc.).
        pub fn should_block(&self, url: &str, source_url: &str, request_type: &str) -> bool {
            match adblock::request::Request::new(url, source_url, request_type) {
                Ok(request) => self.inner.check_network_request(&request).matched,
                Err(_) => false,
            }
        }

        /// Full blocker result for advanced use (redirect, exception, etc.).
        pub fn check_request(
            &self,
            url: &str,
            source_url: &str,
            request_type: &str,
        ) -> Option<adblock::blocker::BlockerResult> {
            adblock::request::Request::new(url, source_url, request_type)
                .ok()
                .map(|req| self.inner.check_network_request(&req))
        }

        /// Serialize the engine to bytes for persistence / caching.
        pub fn serialize(&self) -> Vec<u8> {
            self.inner.serialize()
        }

        /// Deserialize a previously serialized engine.
        pub fn deserialize(data: &[u8]) -> Option<Self> {
            let mut engine = adblock::Engine::default();
            if engine.deserialize(data).is_ok() {
                Some(Self { inner: engine })
            } else {
                None
            }
        }

        /// Wrap in `Arc` for concurrent sharing across threads.
        pub fn into_shared(self) -> Arc<Self> {
            Arc::new(self)
        }
    }
}
