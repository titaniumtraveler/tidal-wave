use crate::{stdio::stdio, ui_state::UiState, usb_device::Device};
use anyhow::{Context, Result};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tokio::io::BufReader;

mod stdio;
mod ui_state;
mod usb_device;

fn main() {
    match try_main().context(io::Error::last_os_error()) {
        Ok(()) => (),
        Err(res) => println!("{res:#?}"),
    }
}

#[tokio::main]
async fn try_main() -> Result<()> {
    let device = Device::try_initialize().await?;
    let state = Arc::new(Mutex::new(UiState::default()));

    stdio(
        device,
        state,
        BufReader::new(tokio::io::stdin()),
        tokio::io::stdout(),
    )
    .await?;
    Ok(())
}
