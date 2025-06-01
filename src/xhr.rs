use crate::trie::Trie;

lazy_static::lazy_static! {
    /// Ignore list of XHR urls for media.
    pub static ref URL_IGNORE_XHR_MEDIA_TRIE: Trie = {
        let mut trie = Trie::new();
        let patterns = [
            "https://www.youtube.com/s/player/",
            "https://www.vimeo.com/player/",
            "https://soundcloud.com/player/",
            "https://open.spotify.com/",
            "https://api.spotify.com/v1/",
            "https://music.apple.com/",
            "https://maps.googleapis.com/"
        ];
        for pattern in &patterns {
            trie.insert(pattern);
        }
        trie
    };

    /// Ignore list of XHR urls.
    pub static ref URL_IGNORE_XHR_TRIE: Trie = {
        let mut trie = Trie::new();
        let patterns = [
            "https://play.google.com/log?",
            "https://googleads.g.doubleclick.net/pagead/id",
            "https://js.monitor.azure.com/scripts",
            "https://securepubads.g.doubleclick.net",
            "https://pixel-config.reddit.com/pixels",
            // amazon product feedback
            "https://www.amazon.com/af/feedback-link?",
            "https://www.google.com/ads/ga-audiences",
            "https://player.vimeo.com/video/",
            "https://www.youtube.com/iframe_api",
            "https://www.youtube.com/youtubei/v1/log_event",
            "https://tr.snapchat.com/config/",
            "https://collect.tealiumiq.com/",
            "https://cdn.acsbapp.com/config/",
            "https://s.yimg.com/wi",
            "https://disney.my.sentry.io/api/",
            "https://www.redditstatic.com/ads",
            "https://logx.optimizely.com/v1/events",
            "https://www.google-analytics.com/g/collect",
            "https://api-2-0.spot.im/v1.0.0/",
            "https://static.hotjar.com/",
            "https://www.youtube.com/youtubei/v1/log_event?alt=json",
            "https://matchadsrvr.yieldmo.com/track/",
            "https://translate.googleapis.com/element/log",
            "https://sentry.io/api/",
            "https://api2.branch.io/",
            "https://i.clean.gg/",
            "https://prebid.media.net/",
            "https://buy.tinypass.com/",
            "https://idx.liadm.com",
            "https://geo.privacymanager.io/",
            "https://nimbleplot.com",
            "https://api.lab.amplitude.com/",
            "https://flag.lab.amplitude.com/sdk/v2/flags",
            "https://api2.amplitude.com/2/httpapi",
            "https://api.data4.net/api/report/send",
            "https://cdn-ukwest.onetrust.com/",
            "https://cdn.onetrust.com/",
            "https://sessions.bugsnag.com/",
            "https://notify.bugsnag.com/",
            "https://geolocation.onetrust.com/",
            "https://assets.adobedtm.com/",
            "https://sdkconfig.pulse.",
            "https://static.criteo.net",
            "https://bat.bing.net",
            "https://fundingchoicesmessages.google.com/",
            "https://api.reviews.io/",
            "https://thearenagroup.sp.spiny.ai/",
            "https://ads.rubiconproject.com/",
            "https://check.analytics.rlcdn.com",
            "https://api.config-security.com/event",
            "https://conf.config-security.com/model",
            "https://pagead2.googlesyndication.com/",
            "https://sumome.com/api/load/",
            "https://ogads-pa.googleapis.com/",
            "https://public-api.wordpress.com/geo/",
            "https://events.api.secureserver.net/",
            "https://csp.secureserver.net/eventbus",
            "https://cdn.optimizely.com/datafiles/",
            "https://ad.doubleclick.net/",
            "https://metrics.beyondwords.io/events",
            "https://rtb.openx.net/openrtbb/prebidjs",
            "https://beacon.taboola.com/",
            "https://collector.ex.co/main/events",
            "https://www.youtube-nocookie.com/youtubei/v1/log_event?",
            "https://api.raygun.io/ping?apiKey=",
            "https://hb.emxdgt.com/",
            "https://token.rubiconproject.com/",
            "https://prebid-server.rubiconproject.com",
            "https://targeting.unrulymedia.com/unruly_prebid",
            "https://bf16218erm.bf.dynatrace.com/",
            "https://firebaselogging-pa.googleapis.com/v1/firelog/",
            "https://prebid.adnxs.com/",
            "https://doh.cq0.co/resolve",
            "https://event.api.drift.com/track",
            "https://aan.amazon.com/cem",
            "https://unagi.amazon.com/1/events/com.amazon.csm.csa.prod",
            "https://vsanalytics.visualsoft.co.uk/com.snowplowanalytics.snowplow/tp2",
            "https://a.klaviyo.com/onsite/track-analytics",
            "https://hit.salesfire.co.uk/config?",
            "https://ib.adnxs.com/",
            "https://secure.adnxs.com/",
            "https://dc.schibsted.io/api/v1/track/",
            "https://live.smartmetrics.co.uk/x/sf",
            "https://api.marker.io/widget/ping",
            "https://targeting.api.drift.com/targeting/evaluate_with_log",
            "https://targeting.api.drift.com/impressions/widget",
            "https://metrics.api.drift.com/monitoring/metrics/",
            "https://events.launchdarkly.com/events/diagnostic/",
            "https://geolocation.onetrust.com/cookieconsentpub/v1/geo/location",
            "https://cdn.segment.",
            ".wixapps.net/api/v1/bulklog",
            "https://error-analytics-sessions-production.shopifysvc.com/",
            "https://static-forms.",
            "https://nhst.tt.omtrdc.net/rest/v1/delivery",
            // video embeddings
            "https://video.squarespace-cdn.com/content/",
            "https://bes.gcp.data.bigcommerce.com/nobot",
            "https://www.youtube.com/youtubei/",
            "http://ec.editmysite.com",
            "https://dcinfos-cache.abtasty.com/",
            "https://featureassets.org/",
            "https://mab.chartbeat.com/",
            "https://c.go-mpulse.net/",
            "https://prodregistryv2.org/v1/",
            "https://dpm.demdex.net/",
            "https://monorail-edge.shopifysvc.com/unstable/produce_batch",
            "https://monorail-edge.shopifysvc.com/v1/produce",
            "https://cloudflareinsights.com/cdn-cgi/rum",
            "https://api.sprig.com/sdk/",
            "googlesyndication.com",
            ".amplitude.com",
            ".doubleclick.net",
            ".doofinder.com",
            ".piano.io/",
            ".browsiprod.com",
            ".onetrust.",
            "https://logs.",
            "/track.php",
            "/api/v1/bulklog",
            "cookieconsentpub",
            "cookie-law-info",
            "mediaelement-and-player.min.j",
            ".ingest.us.sentry.io/",
            "/analytics",
            "/tracking"
        ];
        for pattern in &patterns {
            trie.insert(pattern);
        }
        trie
    };

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_ignore_xhr_trie_contains() {
        // Positive tests - these URLs should be contained in the trie
        let positive_cases = vec![
            "https://play.google.com/log?",
            "https://googleads.g.doubleclick.net/pagead/id",
            ".doubleclick.net",
        ];

        // Negative tests - these URLs should not be contained in the trie
        let negative_cases = vec![
            "https://example.com/track",
            "https://anotherdomain.com/api/",
        ];

        for case in positive_cases {
            assert!(
                URL_IGNORE_XHR_TRIE.contains_prefix(case),
                "Trie should contain: {}",
                case
            );
        }

        for case in negative_cases {
            assert!(
                !URL_IGNORE_XHR_TRIE.contains_prefix(case),
                "Trie should not contain: {}",
                case
            );
        }
    }
}
