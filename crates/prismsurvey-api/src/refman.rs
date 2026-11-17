//! Prism Reference Manager's local API (its docs/local-api.md): each scale's source and adapted-from citations are found
//! in the person's library and formatted there, in APA 7 by default. The API is off until the person turns it on in
//! Prism Reference Manager (Settings, Local connections), listens on 127.0.0.1 only, and answers a client only after
//! the person approves it once in that program's window (pairing). When it is not there, citations stay as the plain
//! APA text the scale already carries (specification, "Place in the Prism suite").
//!
//! The token is kept in Prism Survey's settings folder, encrypted for this Windows user (DPAPI).

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 23119;
const CLIENT_NAME: &str = "Prism Survey";

/// A plain HTTP/1.1 request to 127.0.0.1, with no proxy and no other address possible.
fn http(
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&Value>,
) -> Result<(u16, Value), String> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_millis(800)).map_err(|_| {
        "Prism Reference Manager is not running, or its local connections are turned off."
            .to_string()
    })?;
    s.set_read_timeout(Some(Duration::from_secs(20))).ok();
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", body.len());
    if let Some(t) = token {
        req.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(&body);
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw);
    let (head, rest) = text
        .split_once("\r\n\r\n")
        .ok_or("an answer that was not HTTP")?;
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    // A chunked answer: take the chunks' contents.
    let content = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(rest)
    } else {
        rest.to_string()
    };
    let v: Value = serde_json::from_str(content.trim()).unwrap_or(Value::Null);
    Ok((status, v))
}

fn dechunk(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some((size, after)) = rest.split_once("\r\n") {
        let n = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if n == 0 || after.len() < n {
            break;
        }
        out.push_str(&after[..n]);
        rest = after[n..].trim_start_matches("\r\n");
    }
    out
}

pub struct RefMan {
    pub port: u16,
    token_path: PathBuf,
}

impl RefMan {
    pub fn new(config_dir: &Path, port: u16) -> Self {
        RefMan {
            port,
            token_path: config_dir.join("reference-manager.token"),
        }
    }

    fn token(&self) -> Option<String> {
        let bytes = std::fs::read(&self.token_path).ok()?;
        String::from_utf8(protect::unprotect(&bytes)?).ok()
    }

    fn keep_token(&self, token: &str) -> Result<(), String> {
        if let Some(d) = self.token_path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let sealed =
            protect::protect(token.as_bytes()).ok_or("the token could not be stored safely")?;
        std::fs::write(&self.token_path, sealed).map_err(|e| e.to_string())
    }

    pub fn forget(&self) {
        let _ = std::fs::remove_file(&self.token_path);
    }

    /// Whether the program answers, and whether Prism Survey is paired with it.
    pub fn status(&self) -> Value {
        let reachable = TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], self.port)),
            Duration::from_millis(300),
        )
        .is_ok();
        json!({ "reachable": reachable, "paired": self.token().is_some(), "port": self.port })
    }

    /// Asks to pair; the person approves in Prism Reference Manager's window.
    pub fn pair(&self) -> Result<String, String> {
        let (code, v) = http(
            self.port,
            "POST",
            "/v1/pair",
            None,
            Some(&json!({ "name": CLIENT_NAME, "scopes": ["read", "format", "import"] })),
        )?;
        if code >= 400 {
            return Err(v["error"]
                .as_str()
                .unwrap_or("the pairing request was refused")
                .to_string());
        }
        v["id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "the pairing request was not accepted".into())
    }

    /// The state of a pairing request; on approval the token is stored.
    pub fn pair_status(&self, id: &str) -> Result<String, String> {
        let safe: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let (_, v) = http(self.port, "GET", &format!("/v1/pair/{safe}"), None, None)?;
        let state = v["state"].as_str().unwrap_or("pending").to_string();
        if state == "approved" {
            if let Some(t) = v["token"].as_str() {
                self.keep_token(t)?;
            }
        }
        Ok(state)
    }

    fn call(&self, method: &str, args: Value) -> Result<Value, String> {
        let token = self
            .token()
            .ok_or("Prism Survey is not paired with Prism Reference Manager yet.")?;
        let (code, v) = http(
            self.port,
            "POST",
            "/v1/call",
            Some(&token),
            Some(&json!({ "method": method, "args": args })),
        )?;
        if code == 401 || code == 403 {
            return Err(v["error"]
                .as_str()
                .unwrap_or("Prism Reference Manager refused the request; pair again.")
                .to_string());
        }
        if code >= 400 {
            return Err(v["error"]
                .as_str()
                .unwrap_or("Prism Reference Manager could not do that.")
                .to_string());
        }
        Ok(v["result"].clone())
    }

    /// Finds a reference by DOI, else by title words.
    fn find(&self, doi: Option<&str>, apa: &str) -> Result<Option<String>, String> {
        if let Some(d) = doi.filter(|d| !d.is_empty()) {
            let r = self.call(
                "refs.list",
                json!({ "query": { "search": format!("doi:{d}"), "limit": 3 } }),
            )?;
            if let Some(id) = r["rows"].get(0).and_then(|row| row["id"].as_str()) {
                return Ok(Some(id.to_string()));
            }
        }
        // The title is the part of an APA reference between the year and the next full stop.
        let title = apa
            .split_once(").")
            .map(|(_, t)| t.trim())
            .and_then(|t| t.split(". ").next())
            .unwrap_or("")
            .trim();
        if title.split_whitespace().count() >= 3 {
            let r = self.call(
                "refs.list",
                json!({ "query": { "search": format!("\"{title}\""), "limit": 3 } }),
            )?;
            if let Some(id) = r["rows"].get(0).and_then(|row| row["id"].as_str()) {
                return Ok(Some(id.to_string()));
            }
        }
        Ok(None)
    }

    /// The APA 7 reference for one library record.
    fn format(&self, ref_id: &str) -> Result<String, String> {
        let r = self.call("cite.render", json!({ "request": { "items": [{ "ref_id": ref_id }], "mode": "bibliography", "style": "apa-7", "exact_version": true } }))?;
        let text = r["text"].as_str().unwrap_or("").trim().to_string();
        if text.is_empty() {
            Err("Prism Reference Manager gave no reference text.".into())
        } else {
            Ok(text)
        }
    }

    /// Stages a DOI for review in Prism Reference Manager when the work is not in the library yet.
    fn stage_doi(&self, doi: &str) -> Result<(), String> {
        let found = self.call("lookup.resolve", json!({ "kind": "doi", "id": doi }))?;
        let Some(candidate) = found.get("candidate").filter(|c| !c.is_null()) else {
            return Err(found["message"]
                .as_str()
                .unwrap_or("the DOI was not found")
                .to_string());
        };
        self.call(
            "lookup.stage",
            json!({ "refs": [candidate], "name": "From Prism Survey", "service": "prism-survey" }),
        )?;
        Ok(())
    }

    /// Links and formats one citation. Returns what happened, in words.
    pub fn cite(&self, c: &mut prismsurvey_core::Citation, what: &str) -> String {
        let found = c.reference_manager_id.clone().map(Ok).unwrap_or_else(|| {
            self.find(c.doi.as_deref(), &c.apa)
                .map(|o| o.unwrap_or_default())
        });
        match found {
            Ok(id) if !id.is_empty() => match self.format(&id) {
                Ok(text) => {
                    c.apa = text;
                    c.reference_manager_id = Some(id);
                    format!("The {what} is linked to Prism Reference Manager and formatted in APA 7.")
                }
                Err(e) => format!("The {what} was found but not formatted: {e}"),
            },
            Ok(_) => match c.doi.as_deref() {
                Some(d) => match self.stage_doi(d) {
                    Ok(()) => format!("The {what} is not in your reference library; it was sent to Prism Reference Manager for you to review and accept. Link again afterwards."),
                    Err(e) => format!("The {what} is not in your reference library, and its DOI could not be looked up: {e}"),
                },
                None => format!("The {what} is not in your reference library; it stays as plain APA text."),
            },
            Err(e) => format!("The {what} stays as plain APA text: {e}"),
        }
    }
}

#[cfg(windows)]
mod protect {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    fn run(data: &[u8], seal: bool) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB::default();
        // SAFETY: the input blob points at `data`, which outlives the call; the output is freed with LocalFree.
        unsafe {
            let ok = if seal {
                CryptProtectData(&input, None, None, None, None, 0, &mut out)
            } else {
                CryptUnprotectData(&input, None, None, None, None, 0, &mut out)
            };
            ok.ok()?;
            let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as _)));
            Some(bytes)
        }
    }

    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        run(data, true)
    }

    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        run(data, false)
    }
}

#[cfg(not(windows))]
mod protect {
    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        Some(data.to_vec())
    }
    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        Some(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_answers_are_read() {
        assert_eq!(
            dechunk("5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            "hello world"
        );
    }

    #[test]
    fn a_missing_program_is_a_sentence() {
        // Port 9 on this computer has nothing listening.
        let r = RefMan::new(&std::env::temp_dir(), 9);
        assert_eq!(r.status()["reachable"], false);
        assert!(r.pair().unwrap_err().contains("not running"));
    }

    #[test]
    fn tokens_are_sealed_and_opened() {
        let sealed = protect::protect(b"prm_secret").unwrap();
        assert_eq!(protect::unprotect(&sealed).unwrap(), b"prm_secret");
    }
}
