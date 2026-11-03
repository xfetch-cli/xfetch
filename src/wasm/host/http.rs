//! HTTP host operation.
//!
//! Redirects are disabled at the agent level and followed manually so every
//! hop is re-checked against the `http.allow` manifest patterns; a redirect to
//! a non-allowlisted host is denied. Bodies are read with a hard cap and
//! transported as base64 so binary payloads survive the JSON layer.

use super::{
    HostCallError, HostCallResult, HostContext, parse_bytes, parse_timeout, require_str, to_base64,
};
use serde_json::{Value, json};
use std::io::Read;
use std::time::{Duration, Instant};
use url::Url;

/// Maximum redirect hops before failing the request.
const MAX_REDIRECTS: usize = 5;

/// Performs an HTTP request honoring the policy and deadline.
pub fn request(args: &Value, ctx: &HostContext) -> HostCallResult {
    let method = args
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .to_ascii_uppercase();
    let url = require_str(args, "url")?;
    let headers = parse_headers(args.get("headers"))?;
    let body = parse_bytes(args, "body_base64")?;
    let timeout = parse_timeout(args, ctx)?;

    if !ctx.policy.http_allowed(url) {
        return Err(HostCallError::denied(format!(
            "http request to '{}' is not allowed by the manifest",
            url
        )));
    }

    let mut current = Url::parse(url)
        .map_err(|err| HostCallError::failed(format!("invalid URL '{}': {}", url, err)))?;

    let config = ureq::Agent::config_builder()
        .max_redirects(0)
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build();
    let agent: ureq::Agent = config.into();

    for _ in 0..=MAX_REDIRECTS {
        let started = Instant::now();
        let build = || {
            let mut builder = ureq::http::Request::builder()
                .method(method.as_str())
                .uri(current.as_str());
            for (name, value) in &headers {
                builder = builder.header(name.as_str(), value.as_str());
            }
            builder
        };
        let response = match body.as_deref() {
            Some(bytes) => {
                let request = build().body(bytes).map_err(|err| {
                    HostCallError::failed(format!("invalid HTTP request: {}", err))
                })?;
                agent.run(request)
            }
            None => {
                let request = build().body(()).map_err(|err| {
                    HostCallError::failed(format!("invalid HTTP request: {}", err))
                })?;
                agent.run(request)
            }
        };

        let response = match response {
            Ok(response) => response,
            Err(err) => return Err(transport_error(err.to_string(), started, timeout)),
        };

        let status = response.status().as_u16();
        if (300..400).contains(&status) {
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
                .ok_or_else(|| {
                    HostCallError::failed("redirect response without a Location header")
                })?;
            let next = current.join(&location).map_err(|err| {
                HostCallError::failed(format!("invalid redirect target '{}': {}", location, err))
            })?;
            if !ctx.policy.http_allowed(next.as_str()) {
                return Err(HostCallError::denied(format!(
                    "redirect to '{}' is not allowed by the manifest",
                    next
                )));
            }
            current = next;
            continue;
        }

        return read_response(response, ctx);
    }

    Err(HostCallError::failed("too many redirects"))
}

/// Collects response metadata and the capped body.
fn read_response(
    mut response: ureq::http::Response<ureq::Body>,
    ctx: &HostContext,
) -> HostCallResult {
    let status = response.status().as_u16();

    let mut headers = Vec::new();
    for (name, value) in response.headers() {
        headers.push(json!([
            name.as_str(),
            String::from_utf8_lossy(value.as_bytes()).to_string()
        ]));
    }

    let cap = ctx.policy.host_call_bytes;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .with_config()
        .limit(cap as u64 + 1)
        .reader()
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| HostCallError::failed(format!("failed to read response body: {}", err)))?;

    if bytes.len() > cap {
        return Err(HostCallError::too_large(format!(
            "response body exceeds the {} KiB host_call limit",
            cap / 1024
        )));
    }

    Ok(json!({
        "status": status,
        "headers": headers,
        "body_base64": to_base64(&bytes),
    }))
}

/// Parses the headers field, accepting an object or an array of pairs.
fn parse_headers(value: Option<&Value>) -> Result<Vec<(String, String)>, HostCallError> {
    let mut headers = Vec::new();

    match value {
        None | Some(Value::Null) => {}
        Some(Value::Object(map)) => {
            for (name, value) in map {
                let value = value.as_str().ok_or_else(|| {
                    HostCallError::failed(format!("header '{}' must be a string", name))
                })?;
                headers.push((name.clone(), value.to_string()));
            }
        }
        Some(Value::Array(pairs)) => {
            for pair in pairs {
                let pair = pair.as_array().filter(|p| p.len() == 2).ok_or_else(|| {
                    HostCallError::failed("headers array entries must be [name, value] pairs")
                })?;
                let name = pair[0].as_str().unwrap_or_default();
                let value = pair[1].as_str().unwrap_or_default();
                headers.push((name.to_string(), value.to_string()));
            }
        }
        Some(_) => {
            return Err(HostCallError::failed(
                "headers must be an object or an array of pairs",
            ));
        }
    }

    Ok(headers)
}

/// Classifies a transport failure as timeout when the per-call budget was
/// exhausted, otherwise as a plain failure.
fn transport_error(message: String, started: Instant, timeout: Duration) -> HostCallError {
    let elapsed = started.elapsed();
    if elapsed >= timeout.saturating_sub(Duration::from_millis(100)) {
        HostCallError::timeout(format!("http request timed out after {:?}", timeout))
    } else {
        HostCallError::failed(format!("http request failed: {}", message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::host::{HostContext, HostErrorKind};
    use crate::wasm::manifest::Manifest;
    use crate::wasm::policy::Policy;
    use std::time::{Duration, Instant};

    fn context(json: &str) -> HostContext {
        let manifest: Manifest = serde_json::from_str(json).expect("manifest");
        HostContext {
            policy: Policy::from_manifest(&manifest),
            deadline: Instant::now() + Duration::from_secs(5),
            name: "test".to_string(),
            kind: crate::wasm::GuestKind::Plugin,
        }
    }

    #[test]
    fn parse_headers_accepts_object_and_pairs() {
        let object = parse_headers(Some(&json!({ "accept": "application/json" })));
        assert_eq!(
            object.expect("object"),
            vec![("accept".to_string(), "application/json".to_string())]
        );

        let pairs = parse_headers(Some(&json!([["set-cookie", "a"], ["set-cookie", "b"]])));
        assert_eq!(pairs.expect("pairs").len(), 2);
    }

    #[test]
    fn denied_url_fails_before_network() {
        let ctx = context("{}");
        let err = request(&json!({ "url": "https://example.com" }), &ctx).expect_err("denied");
        assert_eq!(err.kind, HostErrorKind::Denied);
    }

    #[test]
    fn allowed_url_but_connection_failure_is_reported() {
        // Reserved TEST-NET-1 address: allowlisted on purpose, never routable.
        let ctx = context(r#"{ "capabilities": { "http": { "allow": ["http://192.0.2.1/*"] } } }"#);
        let err = request(
            &json!({ "url": "http://192.0.2.1/", "timeout_ms": 200 }),
            &ctx,
        )
        .expect_err("unreachable");
        assert!(matches!(
            err.kind,
            HostErrorKind::Failed | HostErrorKind::Timeout
        ));
    }

    #[test]
    fn successful_request_returns_status_headers_and_body() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Type: text/plain\r\nX-Test: yes\r\n\r\nhello",
                )
                .expect("write");
        });

        let url = format!("http://127.0.0.1:{port}/");
        let allow = format!(r#"{{ "capabilities": {{ "http": {{ "allow": ["{url}"] }} }} }}"#);
        let ctx = context(&allow);
        let result = request(&json!({ "url": url, "timeout_ms": 2000 }), &ctx).expect("request");
        server.join().expect("server");

        assert_eq!(result["status"], 200);
        assert_eq!(result["body_base64"], "aGVsbG8=");
        assert!(result["headers"].as_array().is_some_and(|headers| {
            headers
                .iter()
                .any(|header| header[0] == "x-test" && header[1] == "yes")
        }));
    }
}
