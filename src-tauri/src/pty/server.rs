use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub struct PtyHandle {
    pub port: u16,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
    pub child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
}

pub async fn spawn_pty_server(
    port: u16,
    shell: String,
    cwd: String,
    env: Vec<(String, String)>,
) -> Result<PtyHandle, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&cwd);
    for (key, value) in &env {
        cmd.env(key, value);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "eDEX-UI");

    let child = Arc::new(Mutex::new(
        pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?,
    ));
    let child_for_task = child.clone();

    let master_pty = pair.master;
    let reader = Arc::new(std::sync::Mutex::new(
        master_pty.try_clone_reader().map_err(|e| e.to_string())?,
    ));
    let writer = Arc::new(Mutex::new(
        master_pty.take_writer().map_err(|e| e.to_string())?,
    ));
    let master = Arc::new(Mutex::new(master_pty));

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| e.to_string())?;

    tokio::spawn(async move {
        let result: Result<(), ()> = async {
            let (stream, _) = tokio::select! {
                _ = &mut shutdown_rx => return Ok(()),
                accepted = listener.accept() => accepted.map_err(|_| ())?,
            };

            let ws = accept_async(stream).await.map_err(|_| ())?;
            let (mut ws_tx, mut ws_rx) = ws.split();
            let (pty_tx, mut pty_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
            let reader_task = reader.clone();

            tokio::task::spawn_blocking(move || {
                let mut buf = [0u8; 4096];
                loop {
                    let count = {
                        let mut guard = match reader_task.lock() {
                            Ok(guard) => guard,
                            Err(_) => break,
                        };
                        match guard.read(&mut buf) {
                            Ok(count) if count > 0 => count,
                            _ => break,
                        }
                    };

                    if pty_tx.send(buf[..count].to_vec()).is_err() {
                        break;
                    }
                }
            });

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    maybe_data = pty_rx.recv() => {
                        let Some(data) = maybe_data else { break; };
                        if ws_tx.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    maybe_message = ws_rx.next() => {
                        match maybe_message {
                            Some(Ok(Message::Text(text))) => {
                                if text.starts_with("Resize") && text.len() == 12 {
                                    if let (Ok(cols), Ok(rows)) = (text[6..9].parse::<u16>(), text[9..12].parse::<u16>()) {
                                        let _ = master.lock().await.resize(PtySize {
                                            rows,
                                            cols,
                                            pixel_width: 0,
                                            pixel_height: 0,
                                        });
                                    }
                                } else {
                                    let mut guard = writer.lock().await;
                                    if guard.write_all(text.as_bytes()).is_err() {
                                        break;
                                    }
                                    let _ = guard.flush();
                                }
                            }
                            Some(Ok(Message::Binary(data))) => {
                                let mut guard = writer.lock().await;
                                if guard.write_all(&data).is_err() {
                                    break;
                                }
                                let _ = guard.flush();
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Ok(_)) => {}
                            Some(Err(_)) => break,
                        }
                    }
                }
            }

            Ok(())
        }
        .await;

        let _ = result;
        let mut child = child_for_task.lock().await;
        let _ = child.kill();
        let _ = child.wait();
    });

    Ok(PtyHandle {
        port,
        shutdown_tx,
        child,
    })
}
