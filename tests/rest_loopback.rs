//! Drives `DiscordHttpClient` against a scripted HTTP server on 127.0.0.1.
//!
//! Every test here talks to a local listener only: the client is pointed at it
//! with `with_api_base`. The server records each request it receives and
//! answers from a script, so a test can count how many times one call was
//! sent, which is the whole question for the resend policy: a re-sent POST
//! is a second message.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use discord_user::{
    client::{DiscordHttpClient, ResendPolicy},
    route::Route,
    CreateAttachment, DiscordError,
};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

/// What the server does with one request.
#[derive(Clone)]
enum Reply {
    /// Answer with this status, these extra headers and this body.
    Status(u16, Vec<(&'static str, &'static str)>, &'static str),
    /// Read the request, then close the connection without answering: the
    /// client cannot know whether the request was carried out.
    Hangup,
}

/// One request as the server received it.
#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    /// Lower-cased names, in arrival order, repeats kept.
    headers: Vec<(String, String)>,
}

impl Seen {
    fn header_values(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

/// A scripted server: request N gets `replies[N]`, and every request past the
/// end of the script gets the last reply. Returns the API base to point the
/// client at, and the record of what arrived.
async fn serve(replies: Vec<Reply>) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let record = Arc::clone(&record);
            let replies = replies.clone();
            tokio::spawn(async move {
                let _ = handle(stream, &record, &replies).await;
            });
        }
    });
    (format!("http://{addr}/api/v10"), seen)
}

/// One request per connection: read it, record it, answer it, close.
async fn handle(
    mut stream: TcpStream,
    record: &Mutex<Vec<Seen>>,
    replies: &[Reply],
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let head_end = loop {
        let mut chunk = [0u8; 4096];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or("").split(' ');
    let method = request_line.next().unwrap_or("").to_string();
    let path = request_line.next().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();

    // Read the body so the client is done writing before the answer.
    let length = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok());
    let chunked = headers
        .iter()
        .any(|(k, v)| k == "transfer-encoding" && v.contains("chunked"));
    let mut body = buf[head_end..].to_vec();
    loop {
        let complete = match length {
            Some(len) => body.len() >= len,
            None if chunked => body.ends_with(b"0\r\n\r\n"),
            None => true,
        };
        if complete {
            break;
        }
        let mut chunk = [0u8; 4096];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }

    let index = {
        let mut seen = record.lock().unwrap();
        seen.push(Seen {
            method,
            path,
            headers,
        });
        seen.len() - 1
    };
    let reply = replies
        .get(index)
        .or(replies.last())
        .cloned()
        .unwrap_or(Reply::Hangup);
    match reply {
        Reply::Hangup => stream.shutdown().await,
        Reply::Status(status, extra, text) => {
            let mut out = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
                text.len()
            );
            if !text.is_empty() {
                out.push_str("Content-Type: application/json\r\n");
            }
            for (k, v) in extra {
                out.push_str(&format!("{k}: {v}\r\n"));
            }
            out.push_str("\r\n");
            out.push_str(text);
            stream.write_all(out.as_bytes()).await?;
            stream.shutdown().await
        }
    }
}

fn client(base: &str, policy: ResendPolicy) -> DiscordHttpClient {
    DiscordHttpClient::new("TOKEN", None, false)
        .with_api_base(base)
        .with_resend_policy(policy)
}

fn count(seen: &Mutex<Vec<Seen>>) -> usize {
    seen.lock().unwrap().len()
}

const OK_MESSAGE: &str = r#"{"id":"1"}"#;
const SERVER_ERROR: Reply = Reply::Status(502, Vec::new(), r#"{"message":"bad gateway"}"#);

/// A call can wait out two 2-second retries; nothing here should take longer.
const LIMIT: Duration = Duration::from_secs(20);

// ── POST: never sent twice when the outcome is unknown ─────────────────────

#[tokio::test]
async fn a_post_answered_with_a_server_error_is_sent_once() {
    let (base, seen) = serve(vec![SERVER_ERROR]).await;
    let http = client(&base, ResendPolicy::default());
    let result = timeout(
        LIMIT,
        http.post::<Value, _>(
            Route::CreateMessage { channel_id: 1 },
            json!({ "content": "hi" }),
        ),
    )
    .await
    .unwrap();
    assert!(
        matches!(result, Err(DiscordError::ServiceError { status: 502, .. })),
        "{result:?}"
    );
    assert_eq!(
        count(&seen),
        1,
        "a POST whose outcome is unknown must not be sent again"
    );
}

#[tokio::test]
async fn a_post_whose_connection_drops_is_sent_once() {
    let (base, seen) = serve(vec![Reply::Hangup]).await;
    let http = client(&base, ResendPolicy::default());
    let result = timeout(
        LIMIT,
        http.post::<Value, _>(
            Route::CreateMessage { channel_id: 1 },
            json!({ "content": "hi" }),
        ),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(DiscordError::Http(_))), "{result:?}");
    assert_eq!(
        count(&seen),
        1,
        "the dropped POST may already have been posted"
    );
}

#[tokio::test]
async fn a_patch_answered_with_a_server_error_is_sent_once() {
    let (base, seen) = serve(vec![SERVER_ERROR]).await;
    let http = client(&base, ResendPolicy::default());
    let result = timeout(
        LIMIT,
        http.patch::<Value, _>(Route::EditChannel { channel_id: 1 }, json!({ "name": "x" })),
    )
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(count(&seen), 1);
}

#[tokio::test]
async fn a_multipart_post_whose_connection_drops_is_sent_once() {
    let (base, seen) = serve(vec![Reply::Hangup]).await;
    let http = client(&base, ResendPolicy::default());
    let files = vec![CreateAttachment::bytes("a.txt", b"hello".to_vec())];
    let result = timeout(
        LIMIT,
        http.post_multipart::<Value>(
            Route::CreateMessage { channel_id: 1 },
            json!({ "content": "hi" }),
            files,
        ),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(DiscordError::Http(_))), "{result:?}");
    assert_eq!(count(&seen), 1);
}

// ── idempotent methods: sent again under the default policy only ──────────

#[tokio::test]
async fn a_get_answered_with_a_server_error_is_sent_again() {
    let (base, seen) = serve(vec![
        SERVER_ERROR,
        Reply::Status(200, Vec::new(), OK_MESSAGE),
    ])
    .await;
    let http = client(&base, ResendPolicy::default());
    let result = timeout(
        LIMIT,
        http.get::<Value>(Route::GetChannel { channel_id: 1 }),
    )
    .await
    .unwrap();
    assert_eq!(result.unwrap()["id"], "1");
    assert_eq!(count(&seen), 2);
}

#[tokio::test]
async fn under_never_a_get_answered_with_a_server_error_is_sent_once() {
    let (base, seen) = serve(vec![
        SERVER_ERROR,
        Reply::Status(200, Vec::new(), OK_MESSAGE),
    ])
    .await;
    let http = client(&base, ResendPolicy::Never);
    let result = timeout(
        LIMIT,
        http.get::<Value>(Route::GetChannel { channel_id: 1 }),
    )
    .await
    .unwrap();
    assert!(
        matches!(result, Err(DiscordError::ServiceError { status: 502, .. })),
        "{result:?}"
    );
    assert_eq!(count(&seen), 1);
}

// ── 429: Discord carried nothing out ────────────────────────────────────────

const RATE_LIMITED: Reply = Reply::Status(
    429,
    Vec::new(),
    r#"{"message":"You are being rate limited.","retry_after":0.01,"global":false}"#,
);

#[tokio::test]
async fn a_post_answered_429_is_waited_out_and_sent_again() {
    let (base, seen) = serve(vec![
        RATE_LIMITED,
        Reply::Status(200, Vec::new(), OK_MESSAGE),
    ])
    .await;
    let http = client(&base, ResendPolicy::default());
    let result = timeout(
        LIMIT,
        http.post::<Value, _>(
            Route::CreateMessage { channel_id: 1 },
            json!({ "content": "hi" }),
        ),
    )
    .await
    .unwrap();
    assert_eq!(result.unwrap()["id"], "1");
    assert_eq!(
        count(&seen),
        2,
        "a 429 means it was not posted, so sending again is safe"
    );
}

#[tokio::test]
async fn under_never_a_429_is_returned_not_waited_out() {
    let mut limited = RATE_LIMITED;
    if let Reply::Status(_, ref mut extra, _) = limited {
        extra.push(("retry-after", "7"));
    }
    let (base, seen) = serve(vec![limited, Reply::Status(200, Vec::new(), OK_MESSAGE)]).await;
    let http = client(&base, ResendPolicy::Never);
    let result = timeout(
        LIMIT,
        http.post::<Value, _>(
            Route::CreateMessage { channel_id: 1 },
            json!({ "content": "hi" }),
        ),
    )
    .await
    .unwrap();
    match result {
        Err(DiscordError::RateLimited { retry_after, .. }) => assert_eq!(retry_after, 7.0),
        other => panic!("expected RateLimited, got {other:?}"),
    }
    assert_eq!(count(&seen), 1);
}

#[tokio::test]
async fn under_never_a_multipart_429_is_returned() {
    let (base, seen) = serve(vec![
        RATE_LIMITED,
        Reply::Status(200, Vec::new(), OK_MESSAGE),
    ])
    .await;
    let http = client(&base, ResendPolicy::Never);
    let files = vec![CreateAttachment::bytes("a.txt", b"hello".to_vec())];
    let result = timeout(
        LIMIT,
        http.post_multipart::<Value>(Route::CreateMessage { channel_id: 1 }, json!({}), files),
    )
    .await
    .unwrap();
    assert!(
        matches!(result, Err(DiscordError::RateLimited { .. })),
        "{result:?}"
    );
    assert_eq!(count(&seen), 1);
}

// ── 204 ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_put_answered_204_succeeds() {
    let (base, seen) = serve(vec![Reply::Status(204, Vec::new(), "")]).await;
    let http = client(&base, ResendPolicy::default());
    timeout(
        LIMIT,
        http.put::<(), _>(
            Route::PinMessage {
                channel_id: 1,
                message_id: 2,
            },
            json!({}),
        ),
    )
    .await
    .unwrap()
    .expect("a pin Discord carried out is a success");
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "PUT");
    assert_eq!(seen[0].path, "/api/v10/channels/1/pins/2");
}

// ── the credential ──────────────────────────────────────────────────────────

#[tokio::test]
async fn a_replaced_credential_is_sent_by_the_next_request() {
    let (base, seen) = serve(vec![Reply::Status(200, Vec::new(), OK_MESSAGE)]).await;
    let headers = HashMap::from([
        ("Authorization".to_string(), "FIRST".to_string()),
        ("cookie".to_string(), "cf_clearance=A".to_string()),
    ]);
    let http = DiscordHttpClient::with_headers(headers, None, false)
        .unwrap()
        .with_api_base(&base);
    let clone = http.clone();

    http.get::<Value>(Route::GetChannel { channel_id: 1 })
        .await
        .unwrap();
    http.set_token("SECOND");
    clone
        .get::<Value>(Route::GetChannel { channel_id: 1 })
        .await
        .unwrap();
    assert!(http.replace_headers(HashMap::from([
        ("authorization".to_string(), "THIRD".to_string()),
        ("cookie".to_string(), "cf_clearance=B".to_string())
    ])));
    clone
        .get::<Value>(Route::GetChannel { channel_id: 1 })
        .await
        .unwrap();

    let seen = seen.lock().unwrap();
    let auth: Vec<Vec<&str>> = seen
        .iter()
        .map(|s| s.header_values("authorization"))
        .collect();
    assert_eq!(
        auth,
        vec![vec!["FIRST"], vec!["SECOND"], vec!["THIRD"]],
        "one authorization header per request, the current one"
    );
    let cookie: Vec<Vec<&str>> = seen.iter().map(|s| s.header_values("cookie")).collect();
    assert_eq!(
        cookie,
        vec![
            vec!["cf_clearance=A"],
            vec!["cf_clearance=A"],
            vec!["cf_clearance=B"]
        ],
        "set_token keeps the headers"
    );
}

#[tokio::test]
async fn a_multipart_post_carries_one_content_type_its_own() {
    let (base, seen) = serve(vec![Reply::Status(200, Vec::new(), OK_MESSAGE)]).await;
    let headers = HashMap::from([
        ("authorization".to_string(), "TOKEN".to_string()),
        ("content-type".to_string(), "application/json".to_string()),
    ]);
    let http = DiscordHttpClient::with_headers(headers, None, false)
        .unwrap()
        .with_api_base(&base);
    let files = vec![CreateAttachment::bytes("a.txt", b"hello".to_vec())];
    http.post_multipart::<Value>(
        Route::CreateMessage { channel_id: 1 },
        json!({ "content": "hi" }),
        files,
    )
    .await
    .unwrap();

    let seen = seen.lock().unwrap();
    let types = seen[0].header_values("content-type");
    assert_eq!(types.len(), 1, "{types:?}");
    assert!(
        types[0].starts_with("multipart/form-data; boundary="),
        "{types:?}"
    );
}

#[tokio::test]
async fn a_json_request_keeps_the_configured_content_type() {
    // Not the plain `application/json` reqwest would set by itself, so the
    // test tells "the configured header was sent" from "one was made up".
    let (base, seen) = serve(vec![Reply::Status(200, Vec::new(), OK_MESSAGE)]).await;
    let headers = HashMap::from([
        ("authorization".to_string(), "TOKEN".to_string()),
        (
            "content-type".to_string(),
            "application/json; charset=utf-8".to_string(),
        ),
    ]);
    let http = DiscordHttpClient::with_headers(headers, None, false)
        .unwrap()
        .with_api_base(&base);
    http.post::<Value, _>(
        Route::CreateMessage { channel_id: 1 },
        json!({ "content": "hi" }),
    )
    .await
    .unwrap();
    assert_eq!(
        seen.lock().unwrap()[0].header_values("content-type"),
        vec!["application/json; charset=utf-8"]
    );
}
