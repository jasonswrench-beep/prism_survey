//! Program updates (docs/UPDATES.md), the same design as PrismStatistics' and Prism Reference Manager's.
//!
//! The feed is the public repository's release page: `latest.json` on the newest full release (GitHub's `latest` skips
//! pre-releases), and `previous.json` beside it for **Return to the previous version**, which accepts only an older
//! version. A check is made when the person presses **Check for updates** on the Help tab, or at most once a day if they
//! turned that on (off by default; the window keeps the setting and asks). It sends nothing but what any web request
//! sends: the program's version and the computer's kind. Installing is always a second step that needs the person's
//! go-ahead: the download is checked against a signature made with the maintainer's update key (its public half is built
//! into the program) before anything runs, so a file that doesn't match is refused. The window offers to save unsaved
//! work before it asks for the install.
//!
//! While `PUBKEY` is empty the updater is "not set up" and cannot connect to anything (`tools/make-update-key.bat` puts
//! the key in). The engine (prismsurvey-api) is not involved: the `update.` commands are the window's own, handled in
//! `main.rs`, and the engine makes no network connection for them.

use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Where the newest version is announced: `latest.json` attached to the newest full release.
pub const ENDPOINT: &str = "https://github.com/jasonswrench-beep/prism_survey/releases/latest/download/latest.json";
/// The release before it, for Return to the previous version: `previous.json` on the same release (the build copies
/// the outgoing `latest.json` there). Signed with the same key.
pub const ROLLBACK_ENDPOINT: &str = "https://github.com/jasonswrench-beep/prism_survey/releases/latest/download/previous.json";

/// The public half of the update key (the text of the `.pub` file `tools/make-update-key.ps1` makes). Empty until that
/// is done, and then the updater is "not set up".
pub const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQwNTc5NUFFMjNCNDNBODQKUldTRU9yUWpycFZYUURUcW5GcGF6ZWh5bnRzb1NmQ2kwbmxyZGhHd1ZYNVl2Q1ZrdU1TSi9TaXMK";

/// Where to look and what to trust. The tests make their own, pointed at a server on this computer.
#[derive(Clone, Debug)]
pub struct Feed {
    pub endpoint: String,
    pub pubkey: String,
}

/// Debug builds only: PRISMSURVEY_UPDATE_ENDPOINT, PRISMSURVEY_ROLLBACK_ENDPOINT and PRISMSURVEY_UPDATE_PUBKEY point the
/// program at a test feed. A release build always uses the built-in ones.
fn setting(name: &str, built_in: &str) -> String {
    #[cfg(debug_assertions)]
    if let Ok(v) = std::env::var(name) {
        return v;
    }
    let _ = name;
    built_in.to_string()
}

impl Feed {
    pub fn built_in() -> Feed {
        Feed {
            endpoint: setting("PRISMSURVEY_UPDATE_ENDPOINT", ENDPOINT),
            pubkey: setting("PRISMSURVEY_UPDATE_PUBKEY", PUBKEY),
        }
    }

    /// The feed for Return to the previous version.
    pub fn rollback() -> Feed {
        Feed {
            endpoint: setting("PRISMSURVEY_ROLLBACK_ENDPOINT", ROLLBACK_ENDPOINT),
            pubkey: setting("PRISMSURVEY_UPDATE_PUBKEY", PUBKEY),
        }
    }

    /// True when both the address and the key are present. Without them nothing is ever requested.
    pub fn is_set_up(&self) -> bool {
        !self.endpoint.trim().is_empty() && !self.pubkey.trim().is_empty()
    }

    /// The name of the computer that is asked, for saying so in the window.
    pub fn host(&self) -> String {
        tauri::Url::parse(&self.endpoint)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_default()
    }
}

const NOT_SET_UP: &str = "This copy of Prism Survey is not set up to check for updates. Download new versions from the project's releases page.";

/// What the Help tab shows before anything is asked: the version, who would be asked, and whether it can ask.
pub fn state<R: Runtime>(app: &AppHandle<R>, feed: &Feed, rollback: &Feed) -> Value {
    json!({
        "version": app.package_info().version.to_string(),
        "setUp": feed.is_set_up(),
        "host": feed.host(),
        "rollback": rollback.is_set_up(),
    })
}

/// A rollback accepts only a release older than the running one.
pub fn is_rollback(current: &str, offered: &str) -> bool {
    match (
        semver::Version::parse(current.trim_start_matches('v')),
        semver::Version::parse(offered.trim_start_matches('v')),
    ) {
        (Ok(c), Ok(o)) => o < c,
        _ => false,
    }
}

fn updater<R: Runtime>(app: &AppHandle<R>, feed: &Feed, rollback: bool) -> Result<tauri_plugin_updater::Updater, String> {
    let url = tauri::Url::parse(&feed.endpoint).map_err(|e| format!("The update address is not valid ({e})."))?;
    let mut b = app.updater_builder();
    if rollback {
        b = b.version_comparator(|current, remote| is_rollback(&current.to_string(), &remote.version.to_string()));
    }
    b.pubkey(feed.pubkey.clone())
        .endpoints(vec![url])
        .map_err(|e| e.to_string())?
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

/// Asks the feed whether a version newer than this one exists. The one request, made only when this is called.
pub async fn check_update<R: Runtime>(app: &AppHandle<R>, feed: &Feed) -> Result<Option<Update>, String> {
    let up = updater(app, feed, false)?;
    up.check().await.map_err(|e| format!("Couldn't check for updates: {e}"))
}

/// The Check button.
pub async fn check<R: Runtime>(app: &AppHandle<R>, feed: &Feed) -> Result<Value, String> {
    if !feed.is_set_up() {
        return Err(NOT_SET_UP.into());
    }
    Ok(match check_update(app, feed).await? {
        Some(u) => json!({
            "available": true,
            "version": u.version,
            "notes": u.body.clone().unwrap_or_default(),
            "date": u.date.map(|d| d.to_string()).unwrap_or_default(),
        }),
        None => json!({ "available": false }),
    })
}

/// The Install button: checks again (the feed may have moved on), downloads, verifies the signature and runs the
/// installer, which replaces the program.
pub async fn install<R: Runtime>(app: &AppHandle<R>, feed: &Feed) -> Result<Value, String> {
    if !feed.is_set_up() {
        return Err(NOT_SET_UP.into());
    }
    let up = updater(app, feed, false)?;
    let Some(update) = up.check().await.map_err(|e| format!("Couldn't check for updates: {e}"))? else {
        return Ok(json!({ "installed": false }));
    };
    let handle = app.clone();
    let mut received: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                received += chunk as u64;
                let _ = handle.emit("update-progress", json!({ "received": received, "total": total }));
            },
            || {},
        )
        .await
        .map_err(|e| format!("The update wasn't installed: {e}"))?;
    Ok(json!({ "installed": true, "version": update.version }))
}

/// Return to the previous version: `install` false only says which older version is offered; true downloads it,
/// verifies the signature with the same key and runs its installer.
pub async fn rollback<R: Runtime>(app: &AppHandle<R>, feed: &Feed, install: bool) -> Result<Value, String> {
    if !feed.is_set_up() {
        return Err("Returning to the previous version is not set up in this copy of Prism Survey.".into());
    }
    let up = updater(app, feed, true)?;
    let Some(update) = up.check().await.map_err(|e| format!("Couldn't reach the release page: {e}"))? else {
        return Ok(json!({ "available": false }));
    };
    if !install {
        return Ok(json!({ "available": true, "version": update.version, "notes": update.body.clone().unwrap_or_default() }));
    }
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("The earlier version wasn't installed: {e}"))?;
    Ok(json!({ "installed": true, "version": update.version }))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    // A test-only key pair and a file signed with it (the same fixtures PrismQDA's and PrismStatistics' tests use); never
    // a release key.
    const PUB: &str = include_str!("../tests/fixtures/update-test.pub");
    const OTHER_PUB: &str = include_str!("../tests/fixtures/update-other.pub");
    const SIGNATURE: &str = include_str!("../tests/fixtures/update-setup.bin.sig");
    const SETUP: &[u8] = include_bytes!("../tests/fixtures/update-setup.bin");

    fn app() -> tauri::App<MockRuntime> {
        let mut context = mock_context(noop_assets());
        context
            .config_mut()
            .plugins
            .0
            .insert("updater".into(), json!({ "pubkey": "", "endpoints": [], "windows": { "installMode": "passive" } }));
        mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .expect("the mock app builds")
    }

    struct Server {
        base: String,
        requests: Arc<Mutex<Vec<String>>>,
    }

    /// A feed on this computer: `/latest.json` announces `version`, and `/setup.exe` gives `bytes`.
    fn serve(version: &str, bytes: &'static [u8], status_for_feed: u16) -> Server {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", server.server_addr().to_ip().unwrap().port());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&requests);
        let manifest = json!({
            "version": version,
            "notes": "Fixes for the test.",
            "pub_date": "2026-11-15T12:00:00Z",
            "platforms": { "windows-x86_64": { "signature": SIGNATURE.trim(), "url": format!("{base}/setup.exe") } },
        })
        .to_string();
        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                log.lock().unwrap().push(url.clone());
                let response = if url == "/latest.json" && status_for_feed == 200 {
                    tiny_http::Response::from_string(manifest.clone())
                        .with_header("Content-Type: application/json".parse::<tiny_http::Header>().unwrap())
                        .boxed()
                } else if url == "/setup.exe" {
                    tiny_http::Response::from_data(bytes.to_vec()).boxed()
                } else {
                    tiny_http::Response::from_string("not here").with_status_code(status_for_feed.max(404)).boxed()
                };
                let _ = request.respond(response);
            }
        });
        Server { base, requests }
    }

    fn feed(server: &Server, pubkey: &str) -> Feed {
        Feed { endpoint: format!("{}/latest.json", server.base), pubkey: pubkey.trim().to_string() }
    }

    fn block_on<T>(f: impl std::future::Future<Output = T>) -> T {
        tauri::async_runtime::block_on(f)
    }

    #[test]
    fn without_a_key_or_an_address_nothing_is_ever_asked() {
        let app = app();
        let server = serve("9.9.9", SETUP, 200);
        for f in [
            Feed { endpoint: String::new(), pubkey: PUB.trim().into() },
            Feed { endpoint: format!("{}/latest.json", server.base), pubkey: String::new() },
        ] {
            assert!(!f.is_set_up());
            assert_eq!(state(app.handle(), &f, &f)["setUp"], false);
            assert!(block_on(rollback(app.handle(), &f, false)).unwrap_err().contains("not set up"));
            assert!(block_on(check(app.handle(), &f)).unwrap_err().contains("not set up"));
            assert!(block_on(install(app.handle(), &f)).unwrap_err().contains("not set up"));
        }
        assert!(server.requests.lock().unwrap().is_empty(), "nothing was asked of the server");
        assert_eq!(Feed::built_in().is_set_up(), !PUBKEY.is_empty());
        assert!(Feed::built_in().endpoint.starts_with("https://github.com/"), "the address is a secure one, on GitHub");
        assert!(
            ENDPOINT.ends_with("/releases/latest/download/latest.json")
                && ROLLBACK_ENDPOINT.ends_with("/releases/latest/download/previous.json"),
            "the newest full release's files"
        );
        assert_eq!(Feed::built_in().host(), "github.com");
    }

    #[test]
    fn a_newer_version_is_announced_and_the_same_or_an_older_one_is_not() {
        let app = app();
        let newer = serve("9.9.9", SETUP, 200);
        let found = block_on(check(app.handle(), &feed(&newer, PUB))).unwrap();
        assert_eq!(
            (found["available"].as_bool(), found["version"].as_str(), found["notes"].as_str()),
            (Some(true), Some("9.9.9"), Some("Fixes for the test."))
        );
        assert_eq!(newer.requests.lock().unwrap().as_slice(), ["/latest.json"], "one request, for the feed only");
        let older = serve("0.0.1", SETUP, 200);
        assert_eq!(block_on(check(app.handle(), &feed(&older, PUB))).unwrap()["available"], false);
        let current = app.handle().package_info().version.to_string();
        assert_eq!(
            block_on(check(app.handle(), &feed(&serve(&current, SETUP, 200), PUB))).unwrap()["available"],
            false,
            "this very version is not an update"
        );
    }

    #[test]
    fn rollback_offers_only_an_older_version() {
        assert!(is_rollback("0.2.1", "0.2.0"));
        assert!(!is_rollback("0.2.0", "0.2.0"));
        assert!(!is_rollback("0.2.0", "0.3.0"));
        assert!(!is_rollback("0.2.0", "not a version"));
        let app = app();
        let older = serve("0.0.1", SETUP, 200);
        let offered = block_on(rollback(app.handle(), &feed(&older, PUB), false)).unwrap();
        assert_eq!((offered["available"].as_bool(), offered["version"].as_str()), (Some(true), Some("0.0.1")));
        assert_eq!(older.requests.lock().unwrap().as_slice(), ["/latest.json"], "asking downloads nothing");
        let newer = serve("9.9.9", SETUP, 200);
        assert_eq!(
            block_on(rollback(app.handle(), &feed(&newer, PUB), false)).unwrap()["available"],
            false,
            "a newer version is never a rollback"
        );
    }

    #[test]
    fn a_missing_feed_or_nobody_listening_fails_in_words() {
        let app = app();
        assert!(block_on(check(app.handle(), &feed(&serve("9.9.9", SETUP, 404), PUB)))
            .unwrap_err()
            .starts_with("Couldn't check for updates"));
        let nobody = Feed { endpoint: "http://127.0.0.1:1/latest.json".into(), pubkey: PUB.trim().into() };
        assert!(block_on(check(app.handle(), &nobody)).is_err());
    }

    #[test]
    fn only_the_file_signed_with_the_built_in_key_is_accepted() {
        let app = app();
        let good = serve("9.9.9", SETUP, 200);
        let update = block_on(check_update(app.handle(), &feed(&good, PUB))).unwrap().expect("a newer version");
        assert_eq!(block_on(update.download(|_, _| {}, || {})).expect("the signed file passes"), SETUP);
        let tampered: &'static [u8] = Box::leak({
            let mut b = SETUP.to_vec();
            b[3] ^= 1;
            b.into_boxed_slice()
        });
        let update = block_on(check_update(app.handle(), &feed(&serve("9.9.9", tampered, 200), PUB)))
            .unwrap()
            .expect("announced");
        assert!(block_on(update.download(|_, _| {}, || {})).is_err(), "a changed file is refused");
        let update = block_on(check_update(app.handle(), &feed(&good, OTHER_PUB))).unwrap().expect("announced");
        assert!(block_on(update.download(|_, _| {}, || {})).is_err(), "a signature from another key is refused");
    }
}
