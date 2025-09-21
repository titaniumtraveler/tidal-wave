use crate::{
    ui_state::{Line, UiState},
    usb_device::{Device, Mode},
};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::sleep;

pub async fn stdio<
    R: AsyncBufRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
>(
    device: Device,
    state: Arc<Mutex<UiState>>,
    reader: R,
    writer: W,
) -> Result<()> {
    let stdin = tokio::spawn({
        let device = device.clone();
        let state = Arc::clone(&state);
        async move {
            let mut stdin = reader;
            let mut buf = Vec::new();

            loop {
                let res = async {
                    buf.clear();
                    stdin.read_until(b'\n', &mut buf).await?;
                    let line: Line = serde_json::from_slice(&buf)?;

                    let persistent = line.persistent;
                    let use_cached = line.use_cached;

                    let config = {
                        let config = if !use_cached.unwrap_or(false) {
                            Some(device.read_config(Duration::from_secs(1)).await?)
                        } else {
                            None
                        };

                        let mut state = state.lock().unwrap();
                        if let Some(config) = config {
                            state.cached = config;
                        }

                        state.update_state(line)
                    };

                    device
                        .write_config(
                            &config,
                            match persistent.unwrap_or(false) {
                                true => Mode::Persistant,
                                false => Mode::Temporary,
                            },
                            Duration::from_secs(1),
                        )
                        .await?;
                    anyhow::Ok(())
                }
                .await;

                match res {
                    Ok(()) => {}
                    Err(err) => state.lock().unwrap().io.err = Some(err.to_string()),
                }
            }
        }
    });

    let stdout = tokio::spawn({
        let device = device.clone();
        let state = Arc::clone(&state);
        async move {
            let mut stdout = writer;
            let mut buf = Vec::new();

            loop {
                let res: Result<()> = async {
                    let config = device.read_config(Duration::from_secs(1)).await?;
                    let line = state.lock().unwrap().update_device_info(config);

                    if !line.is_empty() {
                        buf.clear();

                        serde_json::to_writer(&mut buf, &line)?;
                        buf.push(b'\n');

                        stdout.write_all(&buf).await?;
                        stdout.flush().await?;
                    }

                    Ok(())
                }
                .await;

                match res {
                    Ok(()) => {}
                    Err(err) => state.lock().unwrap().io.err = Some(err.to_string()),
                }
                sleep(Duration::from_secs(1)).await
            }
        }
    });

    let (stdin, stdout) = tokio::join!(stdin, stdout);
    stdin?;
    stdout?;

    Ok(())
}
