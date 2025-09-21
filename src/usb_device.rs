use crate::ui_state::Line as UserConfig;
use anyhow::{Context, Result, anyhow};
use nusb::{
    Interface,
    transfer::{ControlIn, ControlOut, ControlType, Recipient},
};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, time::Duration};

#[derive(Clone)]
pub struct Device {
    iface: Interface,
}

impl Device {
    const VENDOR_ID: u16 = 0x0FD9;
    const PRODUCT_ID: u16 = 0x007D;

    pub async fn try_initialize() -> Result<Self> {
        let dev = nusb::list_devices()
            .await?
            .find(|dev| dev.vendor_id() == Self::VENDOR_ID && dev.product_id() == Self::PRODUCT_ID)
            .context("missing device")?;
        let iface = dev
            .interfaces()
            .find(|iface| {
                iface.class() == 0xFF && iface.subclass() == 0xF0 && iface.protocol() == 0x00
            })
            .context("missing interface")?;

        let dev = dev.open().await.context(anyhow!("dev"))?;
        let iface = dev
            .claim_interface(iface.interface_number())
            .await
            .context(anyhow!("iface"))?;

        Ok(Self { iface })
    }

    pub async fn read_config(&self, timeout: Duration) -> Result<DeviceConfiguration> {
        let buf_out = self
            .iface
            .control_in(
                ControlIn {
                    control_type: ControlType::Class,
                    recipient: Recipient::Endpoint,
                    request: 0x0085,
                    value: 0x0000,
                    index: 0x3300,
                    length: 34,
                },
                timeout,
            )
            .await
            .context("read control")?;

        if buf_out.len() != 34 {
            return Err(anyhow!("buffer has wrong size"));
        }

        DeviceConfiguration::read(buf_out.split_first_chunk().context("buffer too short")?.0)
    }

    pub async fn write_config(
        &self,
        config: &DeviceConfiguration,
        mode: Mode,
        timeout: Duration,
    ) -> Result<()> {
        let mut buf = [0; 34];
        config.write(&mut buf);
        self.iface
            .control_out(
                ControlOut {
                    control_type: ControlType::Class,
                    recipient: Recipient::Endpoint,
                    request: 0x0005,
                    value: mode as _,
                    index: 0x3300,
                    data: &buf,
                },
                timeout,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
pub struct DeviceConfiguration {
    /// Input Gain
    ///
    /// Input Gain in dB. Range 0dB to 75dB
    pub gain: u16,

    // Mute
    pub mute: bool,

    /// Clipguard
    pub clipguard: bool,

    /// Phantom Power - 48V Phantom Power
    pub phantom: bool,

    /// Lowcut Filter
    pub lowcut: LowcutFilter,

    /// Monitor Volume
    ///
    /// Monitor volume in dB. Range 0dB to -128dB
    pub volume: i16,

    /// Monitor Mix
    ///
    /// Mix between microphone and PC audio in %
    pub mix: u8,

    /// Mute Color
    pub color_mute: Color,

    /// General Color
    ///
    /// For some reason they appear *trice as part of the config bytes
    pub color_gen: Color,

    /// Wave Gain Lock
    pub gain_lock: bool,

    /// Gain Reduction Color
    pub color_gain_reduction: Color,

    /// Clipguard Indicator
    pub clipguard_indicator: bool,

    /// Low Impedence Mode
    pub lim: bool,
}

impl DeviceConfiguration {
    fn read(buf: &[u8; 34]) -> Result<Self> {
        Ok(Self {
            gain: read_field::<0, 2, _>(buf, u16::from_le_bytes),
            mute: read_bool::<4, 1>(buf)?,
            clipguard: read_bool::<5, 1>(buf)?,
            phantom: read_bool::<6, 1>(buf)?,
            lowcut: try_read_field::<7, 2, _, _>(buf, "Lowcut Filter", |data| {
                match u16::from_le_bytes(data) {
                    0x0000 => Ok(LowcutFilter::Off),
                    0x0001 => Ok(LowcutFilter::Cutoff080Hz),
                    0x0100 => Ok(LowcutFilter::Cutoff120Hz),
                    err => Err(err),
                }
            })?,
            volume: read_field::<9, 2, _>(buf, i16::from_le_bytes),
            mix: read_field::<13, 1, _>(buf, u8::from_le_bytes),
            color_mute: Color::read::<15, 3>(buf),
            color_gen: Color::read::<18, 9>(buf),
            gain_lock: read_bool::<28, 1>(buf)?,
            color_gain_reduction: Color::read::<29, 3>(buf),
            clipguard_indicator: read_bool::<32, 1>(buf)?,
            lim: read_bool::<33, 1>(buf)?,
        })
    }

    fn write(&self, buf: &mut [u8; 34]) {
        write_field::<0, 2>(buf, self.gain.to_le_bytes());
        write_field::<2, 2>(buf, [0, 0xec]);
        write_field::<4, 1>(buf, [self.mute as u8]);
        write_field::<5, 1>(buf, [self.clipguard as u8]);
        write_field::<6, 1>(buf, [self.phantom as u8]);
        write_field::<7, 2>(buf, (self.lowcut as u16).to_le_bytes());
        write_field::<9, 2>(buf, self.volume.to_le_bytes());
        write_field::<11, 1>(buf, [0u8]);

        // Who knows why this is in the protocol, but it is inside of there apparently *shrug*
        write_field::<12, 1>(
            buf,
            [match self.mix {
                41 | 47 => 0b0000_0001,
                _ => 0b0000_0000,
            }],
        );

        write_field::<13, 1>(buf, self.mix.to_le_bytes());
        write_field::<14, 1>(buf, [0b0000_0001]);
        write_field::<15, 3>(buf, self.color_mute.0);

        // For some reasons the protocol includes the base color three times
        write_field::<18, 3>(buf, self.color_gen.0);
        write_field::<21, 3>(buf, self.color_gen.0);
        write_field::<24, 3>(buf, self.color_gen.0);

        write_field::<27, 1>(buf, [0b0000_0001]);

        write_field::<28, 1>(buf, [self.gain_lock as u8]);
        write_field::<29, 3>(buf, self.color_gain_reduction.0);
        write_field::<32, 1>(buf, [self.clipguard_indicator as u8]);
        write_field::<33, 1>(buf, [self.lim as u8]);
    }

    pub fn merge(&mut self, user_config: &UserConfig) {
        let UserConfig {
            gain,
            mute,
            clipguard,
            phantom,
            lowcut,
            volume,
            mix,
            color_mute,
            color_gen,
            gain_lock,
            color_gain_reduction,
            clipguard_indicator,
            lim,
            persistent: _,
            use_cached: _,
            err: _,
        } = user_config;

        if let Some(gain) = gain {
            self.gain = *gain;
        }
        if let Some(mute) = mute {
            self.mute = *mute;
        }
        if let Some(clipguard) = clipguard {
            self.clipguard = *clipguard;
        }
        if let Some(phantom) = phantom {
            self.phantom = *phantom;
        }
        if let Some(lowcut) = lowcut {
            self.lowcut = *lowcut;
        }
        if let Some(volume) = volume {
            self.volume = *volume;
        }
        if let Some(mix) = mix {
            self.mix = *mix;
        }
        if let Some(color_mute) = color_mute {
            self.color_mute = *color_mute;
        }
        if let Some(color_gen) = color_gen {
            self.color_gen = *color_gen;
        }
        if let Some(gain_lock) = gain_lock {
            self.gain_lock = *gain_lock;
        }
        if let Some(color_gain_reduction) = color_gain_reduction {
            self.color_gain_reduction = *color_gain_reduction;
        }
        if let Some(clipguard_indicator) = clipguard_indicator {
            self.clipguard_indicator = *clipguard_indicator;
        }
        if let Some(lim) = lim {
            self.lim = *lim;
        }
    }
}

fn read_field<const OFFSET: usize, const LEN: usize, T>(
    buf: &[u8; 34],
    f: impl FnOnce([u8; LEN]) -> T,
) -> T {
    let data = *buf
        .get(OFFSET..(OFFSET + LEN))
        .expect("failed to get slice")
        .first_chunk()
        .expect("couldn't get array");
    f(data)
}

fn try_read_field<const OFFSET: usize, const LEN: usize, T, E: Display>(
    buf: &[u8; 34],
    typ: &str,
    f: impl FnOnce([u8; LEN]) -> std::result::Result<T, E>,
) -> Result<T> {
    let res = read_field::<OFFSET, LEN, _>(buf, f);
    match res {
        Ok(ok) => Ok(ok),
        Err(err) => Err(anyhow!("expected {typ} at {OFFSET}:{LEN} got {err}")),
    }
}

fn read_bool<const OFFSET: usize, const LEN: usize>(buf: &[u8; 34]) -> Result<bool> {
    try_read_field::<OFFSET, 1, _, _>(buf, "bool", |b| match u8::from_be_bytes(b) {
        0b0000_0000 => Ok(false),
        0b0000_0001 => Ok(true),
        err => Err(err),
    })
}

fn write_field<const OFFSET: usize, const LEN: usize>(buf: &mut [u8; 34], src: [u8; LEN]) {
    let buf: &mut [u8; LEN] = buf
        .get_mut(OFFSET..(OFFSET + LEN))
        .expect("failed to get slice")
        .first_chunk_mut()
        .expect("couldn't get array");

    buf.copy_from_slice(&src);
}

#[repr(u16)]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LowcutFilter {
    #[default]
    Off = 0x0000,
    Cutoff080Hz = 0x0100,
    Cutoff120Hz = 0x0001,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color([u8; 3]);

impl Color {
    fn read<const OFFSET: usize, const LEN: usize>(buf: &[u8; 34]) -> Self {
        read_field::<OFFSET, 3, _>(buf, Color)
    }
}

#[repr(u16)]
pub enum Mode {
    Temporary = 0x0000,
    Persistant = 0x0002,
}
