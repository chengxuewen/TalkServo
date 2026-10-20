#![cfg(feature = "stub-media")] // test_bridge is stub-gated (ws_flow convention)
// plan-7 S7.6 / modules-05 E12: SERVER-PROCESS crash recovery. The contract:
// secrets live outside the process (identity env / here: the env we re-supply),
// rooms are lazily rebuilt from nothing, clients rejoin with their pre-crash
// tokens, and the floor works again immediately. Generation seeds from the
// clock so a rebuilt room can't collide with its old frames.
use futures_util::SinkExt;
use serde_json::{Value, json};
use talkservo_server::test_bridge;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message as WsMsg;

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const SECRET: &str = "crash-recovery-secret";
const ROOM: &str = "crash-room";

async fn http_healthz(port: u16) -> bool {
    let mut stream = match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
        Ok(s) => s,
        Err(_) => return false,
    };
    let req = "GET /healthz HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_to_end(&mut buf),
    )
    .await;
    String::from_utf8_lossy(&buf).contains("200")
}

/// spawn the real binary (env-only `run` mode); port races after a kill are
/// retried (child exits fast on bind failure)
async fn spawn_server(port: u16) -> std::process::Child {
    let bin = env!("CARGO_BIN_EXE_talkservo-server");
    let logfile = format!("/tmp/ts-crash-{port}.log");
    for attempt in 0..10u32 {
        let errfile = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&logfile)
            .unwrap();
        let mut child = std::process::Command::new(bin)
            .arg("run")
            .env("TS_JWT_SECRET", SECRET)
            .env("TS_BIND", format!("127.0.0.1:{port}"))
            // no admin surface needed here; keep secrets env-only
            .env_remove("TS_ADMIN_SECRET")
            .stdout(errfile.try_clone().unwrap())
            .stderr(errfile)
            .spawn()
            .expect("spawn talkservo-server");
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if http_healthz(port).await {
                return child;
            }
        }
        // bind raced or slow start: kill this candidate and retry
        let _ = child.kill();
        let _ = child.wait();
        tokio::time::sleep(std::time::Duration::from_millis(200 * (attempt as u64 + 1))).await;
    }
    panic!("server never came up on :{port}");
}

fn token(sub: &str) -> String {
    test_bridge::issue_token(SECRET, sub, ROOM, talkservo_core::wire::Role::Field, 3600)
}

async fn join(sub: &str) -> (Ws, String) {
    let port = last_port().await;
    let jwt = token(sub);
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/ws"))
        .await
        .expect("connect");
    ws.send(WsMsg::Text(
        json!({"type": "join", "v": 1, "jwt": jwt})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    (ws, jwt)
}

static PORT: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
async fn last_port() -> u16 {
    if let Some(p) = PORT.get() {
        return *p;
    }
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let p = l.local_addr().unwrap().port();
    drop(l);
    let _ = PORT.set(p);
    p
}

async fn recv_type(ws: &mut Ws, want: &str) -> Value {
    use futures_util::StreamExt;
    for _ in 0..24 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .expect("frame timeout")
            .expect("open")
            .expect("text");
        let v: Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
        if v["type"] == want {
            return v;
        }
    }
    panic!("never saw {want}");
}

async fn drain_burst(ws: &mut Ws) {
    use futures_util::StreamExt;
    // burst length is session-dependent (solo rejoin gets 3 frames, joined
    // room 4) - drain with a short quiet-period timeout, never block on a
    // frame that will not come (measured: hang at 20min)
    for _ in 0..6 {
        if tokio::time::timeout(std::time::Duration::from_millis(400), ws.next())
            .await
            .is_err()
        {
            break;
        }
    }
}

#[tokio::test]
async fn server_crash_respawn_clients_rejoin_with_old_tokens() {
    let port = last_port().await;
    let mut child = spawn_server(port).await;

    // pre-crash session: a holds the floor, b listens
    let (mut a, jwt_a) = join("alice").await;
    drain_burst(&mut a).await;
    let (mut b, _jwt_b) = join("bob").await;
    drain_burst(&mut b).await;
    a.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    recv_type(&mut a, "floor_granted").await;

    // ---- kill -9 the server process (no graceful exit) ----
    child.kill().expect("kill");
    let _ = child.wait();

    // ---- respawn (same secrets, same port - oxmgr's job in production) ----
    let mut child2 = spawn_server(port).await;

    // old token still valid (identity env survives the process); room rebuilt
    let (mut a2, _) = {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/ws"))
            .await
            .expect("reconnect");
        ws.send(WsMsg::Text(
            json!({"type": "join", "v": 1, "jwt": jwt_a})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
        (ws, jwt_a)
    };
    let welcome = recv_type(&mut a2, "welcome").await;
    assert_eq!(welcome["peer_id"], "alice", "identity rebuilds after crash");
    drain_burst(&mut a2).await;

    // floor works again from zero (grants empty, request grants fresh)
    a2.send(WsMsg::Text(
        json!({"type": "floor_request", "priority": 0, "preempt": false})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let granted = recv_type(&mut a2, "floor_granted").await;
    assert_eq!(granted["grants"], json!(["alice"]));

    // second client rejoins and sees the peer roster rebuild
    let mut b2 = {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/ws"))
            .await
            .expect("reconnect b");
        ws.send(WsMsg::Text(
            json!({"type": "join", "v": 1, "jwt": token("bob")})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
        ws
    };
    recv_type(&mut b2, "welcome").await;
    // late joiner learns of alice via the role-scoped PEER LIST (PeerJoined
    // deltas only reach already-present members - modules/02 J-sequence)
    let plist = recv_type(&mut b2, "peer_list").await;
    assert!(
        plist["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "alice"),
        "alice in rebuilt roster: {plist}"
    );

    // cleanup
    drop(a2);
    drop(b2);
    child2.kill().expect("kill2");
    let _ = child2.wait();
}
