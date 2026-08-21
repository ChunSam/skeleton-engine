//! Echo server for the `wasm_failpaths` browser check.
//!
//! ```text
//! cargo run --example wasm_failpaths_echo_server
//! ```
//!
//! Deliberately trivial: it echoes every text/binary frame straight back. That is all the
//! `wasm_failpaths` page needs, because the property under test is **"did the message arrive at
//! all"** — specifically a message sent while the socket was still `CONNECTING`, which the web
//! client silently dropped until v0.150.2. An echo turns that into something the page can observe
//! for itself: send before open, and see it come back.
//!
//! ⚠️ **An echo is required, not a convenience.** `netplay_server` also receives a pre-open message
//! (`ClientMsg::Join`) but ignores its content, so nothing about its arrival is observable from the
//! page — which is precisely how the v0.150.2 bug survived a tree full of networked examples. The
//! property needs a server that says *"I got exactly this"*.
//!
//! It also prints every frame it receives, so the smoke script has a second, independent witness in
//! the server log if the page's own verdict never appears.
//!
//! `FAILPATHS_ADDR` overrides the listen address (default `127.0.0.1:9007`), matching the
//! `<NAME>_ADDR` convention the networked examples already use. `9007` is clear of
//! `netplay_server`'s `9006` so both can run at once.
//!
//! NATIVE_ONLY: it is a TCP server — tungstenite and std::net are native-only by construction
//!
//! (That line is read by `scripts/build_wasm_examples.sh`, which checks it **both ways**: an
//! undeclared wasm failure fails, and a declaration on something that does build also fails.)

use std::net::TcpListener;
use std::thread;

fn main() {
    let addr = std::env::var("FAILPATHS_ADDR").unwrap_or_else(|_| "127.0.0.1:9007".to_string());
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("echo server: bind {addr} failed: {e}");
            std::process::exit(2);
        }
    };
    // The smoke waits for this line before pointing a browser at us.
    println!("echo server listening on {addr}");

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        thread::spawn(move || {
            let mut ws = match tungstenite::accept(stream) {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("echo server: handshake failed: {e}");
                    return;
                }
            };
            println!("client connected");
            loop {
                match ws.read() {
                    Ok(msg @ tungstenite::Message::Text(_))
                    | Ok(msg @ tungstenite::Message::Binary(_)) => {
                        match &msg {
                            tungstenite::Message::Text(t) => println!("recv text: {t}"),
                            tungstenite::Message::Binary(b) => println!("recv binary: {}", b.len()),
                            _ => unreachable!(),
                        }
                        if ws.send(msg).is_err() {
                            return;
                        }
                    }
                    Ok(tungstenite::Message::Close(_)) => {
                        println!("client disconnected");
                        return;
                    }
                    // Ping/Pong/Frame are handled inside tungstenite.
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("echo server: read ended: {e}");
                        return;
                    }
                }
            }
        });
    }
}
