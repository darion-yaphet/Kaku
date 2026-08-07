//! Shared OSC 7 cwd URL parsing for AI surfaces (Cmd+L overlay and `k` CLI).
//!
//! Overlay and CLI both need to know when a pane's cwd belongs to a remote
//! host so local tools can be disabled consistently.

/// Split an OSC 7 cwd URL into a path plus the remote host, if any.
///
/// OSC 7 emits `file://<hostname>/path`; a host that is neither empty,
/// `localhost`, nor this machine means the pane is inside an ssh (or other
/// remote) session and the path must not be treated as local. Hostnames are
/// compared case-insensitively and by first label, because the remote side
/// may report a short name while macOS reports `name.local` (or vice versa).
pub fn split_cwd_url(url: &url::Url, local_host: &str) -> (String, Option<String>) {
    let path = url.path().to_string();
    let host = url.host_str().unwrap_or("");
    if url.scheme() != "file" {
        return (path, Some(host.to_string()));
    }
    if host.is_empty() || host.eq_ignore_ascii_case("localhost") {
        return (path, None);
    }
    let first_label = |s: &str| s.split('.').next().unwrap_or(s).to_ascii_lowercase();
    if !local_host.is_empty() && first_label(host) == first_label(local_host) {
        return (path, None);
    }
    (path, Some(host.to_string()))
}

pub fn local_hostname() -> String {
    static HOSTNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    HOSTNAME
        .get_or_init(|| {
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_default()
        })
        .clone()
}

/// Parse a cwd URL string (OSC 7 style) against this machine's hostname.
///
/// Used by `cli_chat` (lib / `k` binary). The GUI binary prefers `Url` values
/// from the mux, so this helper may look unused there.
#[allow(dead_code)]
pub fn split_cwd_url_str(url: &str) -> Option<(String, Option<String>)> {
    let parsed = url::Url::parse(url).ok()?;
    Some(split_cwd_url(&parsed, &local_hostname()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd(url: &str, local: &str) -> (String, Option<String>) {
        split_cwd_url(&url::Url::parse(url).unwrap(), local)
    }

    #[test]
    fn local_cwd_urls_have_no_remote_host() {
        assert_eq!(cwd("file:///Users/a", "mac"), ("/Users/a".into(), None));
        assert_eq!(
            cwd("file://localhost/Users/a", "mac"),
            ("/Users/a".into(), None)
        );
        assert_eq!(
            cwd("file://mac.local/Users/a", "mac"),
            ("/Users/a".into(), None)
        );
        assert_eq!(
            cwd("file://MAC/Users/a", "mac.local"),
            ("/Users/a".into(), None)
        );
    }

    #[test]
    fn remote_cwd_urls_report_the_host() {
        assert_eq!(
            cwd("file://server/home/u", "mac.local"),
            ("/home/u".into(), Some("server".into()))
        );
        assert_eq!(
            cwd("file://build.corp.example/srv", "mac"),
            ("/srv".into(), Some("build.corp.example".into()))
        );
    }

    #[test]
    fn non_file_scheme_is_treated_as_remote() {
        let (_, host) = cwd("ftp://server/pub", "server");
        assert_eq!(host, Some("server".into()));
    }
}
