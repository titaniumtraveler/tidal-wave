use anyhow::{Context, Result, anyhow};
use nusb::{
    Interface,
    transfer::{ControlIn, ControlOut, ControlType, Recipient},
};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, io, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    time::sleep,
};

fn main() {
    match try_main().context(io::Error::last_os_error()) {
        Ok(()) => (),
        Err(res) => println!("{res:#?}"),
    }
}

#[tokio::main]
async fn try_main() -> Result<()> {
    let dev = nusb::list_devices()
        .await?
        .find(|dev| dev.vendor_id() == 0x0FD9 && dev.product_id() == 0x007D)
        .context("missing device")?;
    let iface = dev
        .interfaces()
        .find(|iface| iface.class() == 0xFF && iface.subclass() == 0xF0 && iface.protocol() == 0x00)
        .context("missing interface")?;

    let dev = dev.open().await.context(anyhow!("dev"))?;
    let iface = dev
        .claim_interface(iface.interface_number())
        .await
        .context(anyhow!("iface"))?;

    let stdin = tokio::spawn({
        let iface = iface.clone();
        async move {
            let mut stdin = BufReader::new(tokio::io::stdin());
            let mut buf = Vec::new();
            let mut config = Config::default();

            loop {
                let res = async {
                    buf.clear();
                    stdin.read_until(b'\n', &mut buf).await?;
                    let user_config: UserConfig = serde_json::from_slice(&buf)?;

                    if !user_config.use_cached.unwrap_or(false) {
                        config = read_config(&iface).await?;
                    }

                    config.merge(&user_config);

                    write_config(
                        &iface,
                        &config,
                        match user_config.persistant.unwrap_or(false) {
                            true => Mode::Persistant,
                            false => Mode::Temporary,
                        },
                    )
                    .await?;
                    anyhow::Ok(())
                }
                .await;

                match res {
                    Ok(()) => {}
                    Err(err) => eprintln!("{err}"),
                }
            }
        }
    });

    let stdout = tokio::spawn({
        let iface = iface.clone();
        async move {
            let mut stdout = BufWriter::new(tokio::io::stdout());
            let mut buf = Vec::new();
            let mut user_config = UserConfig::default();

            loop {
                let res: Result<()> = async {
                    let config = read_config(&iface).await?.diff(&mut user_config);

                    if !config.is_empty() {
                        buf.clear();

                        serde_json::to_writer(&mut buf, &config)?;
                        buf.push(b'\n');

                        stdout.write_all(&buf).await?;
                        stdout.flush().await?;
                    }

                    Ok(())
                }
                .await;

                match res {
                    Ok(()) => {}
                    Err(err) => eprintln!("{err}"),
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

async fn read_config(interface: &Interface) -> Result<Config> {
    let buf_out = interface
        .control_in(
            ControlIn {
                control_type: ControlType::Class,
                recipient: Recipient::Endpoint,
                request: 0x0085,
                value: 0x0000,
                index: 0x3300,
                length: 34,
            },
            Duration::from_secs(1),
        )
        .await
        .context("read control")?;

    if buf_out.len() != 34 {
        return Err(anyhow!("buffer has wrong size"));
    }

    Config::read(buf_out.split_first_chunk().context("buffer too short")?.0)
}

async fn write_config(interface: &Interface, config: &Config, mode: Mode) -> Result<()> {
    let mut buf = [0; 34];
    config.write(&mut buf);
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Class,
                recipient: Recipient::Endpoint,
                request: 0x0005,
                value: mode as _,
                index: 0x3300,
                data: &buf,
            },
            Duration::from_secs(1),
        )
        .await?;
    Ok(())
}

#[repr(u16)]
enum Mode {
    Temporary = 0x0000,
    Persistant = 0x0002,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UserConfig {
    /// Input Gain
    ///
    /// Input Gain in dB. Range 0dB to 75dB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gain: Option<u16>,

    // Mute
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mute: Option<bool>,

    /// Clipguard
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clipguard: Option<bool>,

    /// Phantom Power - 48V Phantom Power
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phantom: Option<bool>,

    /// Lowcut Filter
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lowcut: Option<LowcutFilter>,

    /// Monitor Volume
    ///
    /// Monitor volume in dB. Range 0dB to -128dB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<i16>,

    /// Monitor Mix
    ///
    /// Mix between microphone and PC audio in %
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mix: Option<u8>,

    /// Mute Color
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color_mute: Option<Color>,

    /// General Color
    ///
    /// For some reason they appear *trice as part of the config bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color_gen: Option<Color>,

    /// Wave Gain Lock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gain_lock: Option<bool>,

    /// Gain Reduction Color
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color_gain_reduction: Option<Color>,

    /// Clipguard Indicator
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clipguard_indicator: Option<bool>,

    /// Low Impedence Mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lim: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    persistant: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    use_cached: Option<bool>,
}

impl UserConfig {
    fn is_empty(&self) -> bool {
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
            persistant: _,
            use_cached: _,
        } = &self;

        gain.is_none()
            && mute.is_none()
            && clipguard.is_none()
            && phantom.is_none()
            && lowcut.is_none()
            && volume.is_none()
            && mix.is_none()
            && color_mute.is_none()
            && color_gen.is_none()
            && gain_lock.is_none()
            && color_gain_reduction.is_none()
            && clipguard_indicator.is_none()
            && lim.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Config {
    /// Input Gain
    ///
    /// Input Gain in dB. Range 0dB to 75dB
    gain: u16,

    // Mute
    mute: bool,

    /// Clipguard
    clipguard: bool,

    /// Phantom Power - 48V Phantom Power
    phantom: bool,

    /// Lowcut Filter
    lowcut: LowcutFilter,

    /// Monitor Volume
    ///
    /// Monitor volume in dB. Range 0dB to -128dB
    volume: i16,

    /// Monitor Mix
    ///
    /// Mix between microphone and PC audio in %
    mix: u8,

    /// Mute Color
    color_mute: Color,

    /// General Color
    ///
    /// For some reason they appear *trice as part of the config bytes
    color_gen: Color,

    /// Wave Gain Lock
    gain_lock: bool,

    /// Gain Reduction Color
    color_gain_reduction: Color,

    /// Clipguard Indicator
    clipguard_indicator: bool,

    /// Low Impedence Mode
    lim: bool,
}

impl Config {
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

    fn merge(&mut self, user_config: &UserConfig) {
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
            persistant: _,
            use_cached: _,
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

    fn diff(&self, config: &mut UserConfig) -> UserConfig {
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
            persistant: _,
            use_cached: _,
        } = config;

        UserConfig {
            gain: match gain {
                Some(gain) if self.gain != *gain => {
                    *gain = self.gain;
                    Some(self.gain)
                }
                None => {
                    *gain = Some(self.gain);
                    Some(self.gain)
                }
                _ => None,
            },
            mute: match mute {
                Some(mute) if self.mute != *mute => {
                    *mute = self.mute;
                    Some(self.mute)
                }
                None => {
                    *mute = Some(self.mute);
                    Some(self.mute)
                }
                _ => None,
            },
            clipguard: match clipguard {
                Some(clipguard) if self.clipguard != *clipguard => {
                    *clipguard = self.clipguard;
                    Some(self.clipguard)
                }
                None => {
                    *clipguard = Some(self.clipguard);
                    Some(self.clipguard)
                }
                _ => None,
            },
            phantom: match phantom {
                Some(phantom) if self.phantom != *phantom => {
                    *phantom = self.phantom;
                    Some(self.phantom)
                }
                None => {
                    *phantom = Some(self.phantom);
                    Some(self.phantom)
                }
                _ => None,
            },
            lowcut: match lowcut {
                Some(lowcut) if self.lowcut != *lowcut => {
                    *lowcut = self.lowcut;
                    Some(self.lowcut)
                }
                None => {
                    *lowcut = Some(self.lowcut);
                    Some(self.lowcut)
                }
                _ => None,
            },
            volume: match volume {
                Some(volume) if self.volume != *volume => {
                    *volume = self.volume;
                    Some(self.volume)
                }
                None => {
                    *volume = Some(self.volume);
                    Some(self.volume)
                }
                _ => None,
            },
            mix: match mix {
                Some(mix) if self.mix != *mix => {
                    *mix = self.mix;
                    Some(self.mix)
                }
                None => {
                    *mix = Some(self.mix);
                    Some(self.mix)
                }
                _ => None,
            },
            color_mute: match color_mute {
                Some(color_mute) if self.color_mute != *color_mute => {
                    *color_mute = self.color_mute;
                    Some(self.color_mute)
                }
                None => {
                    *color_mute = Some(self.color_mute);
                    Some(self.color_mute)
                }
                _ => None,
            },
            color_gen: match color_gen {
                Some(color_gen) if self.color_gen != *color_gen => {
                    *color_gen = self.color_gen;
                    Some(self.color_gen)
                }
                None => {
                    *color_gen = Some(self.color_gen);
                    Some(self.color_gen)
                }
                _ => None,
            },
            gain_lock: match gain_lock {
                Some(gain_lock) if self.gain_lock != *gain_lock => {
                    *gain_lock = self.gain_lock;
                    Some(self.gain_lock)
                }
                None => {
                    *gain_lock = Some(self.gain_lock);
                    Some(self.gain_lock)
                }
                _ => None,
            },
            color_gain_reduction: match color_gain_reduction {
                Some(color_gain_reduction)
                    if self.color_gain_reduction != *color_gain_reduction =>
                {
                    *color_gain_reduction = self.color_gain_reduction;
                    Some(self.color_gain_reduction)
                }
                None => {
                    *color_gain_reduction = Some(self.color_gain_reduction);
                    Some(self.color_gain_reduction)
                }
                _ => None,
            },
            clipguard_indicator: match clipguard_indicator {
                Some(clipguard_indicator) if self.clipguard_indicator != *clipguard_indicator => {
                    *clipguard_indicator = self.clipguard_indicator;
                    Some(self.clipguard_indicator)
                }
                None => {
                    *clipguard_indicator = Some(self.clipguard_indicator);
                    Some(self.clipguard_indicator)
                }
                _ => None,
            },
            lim: match lim {
                Some(lim) if self.lim != *lim => {
                    *lim = self.lim;
                    Some(self.lim)
                }
                None => {
                    *lim = Some(self.lim);
                    Some(self.lim)
                }
                _ => None,
            },
            persistant: None,
            use_cached: None,
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
struct Color([u8; 3]);

impl Color {
    fn read<const OFFSET: usize, const LEN: usize>(buf: &[u8; 34]) -> Self {
        read_field::<OFFSET, 3, _>(buf, Color)
    }
}

#[test]
#[allow(clippy::bool_assert_comparison)]
fn merge_user_config() {
    let mut config = Config::default();
    let user_config = UserConfig {
        gain: Some(30 << 8),
        mute: Some(true),
        ..UserConfig::default()
    };

    config.merge(&user_config);
    assert_eq!(30 << 8, config.gain);
    assert_eq!(true, config.mute);
}
