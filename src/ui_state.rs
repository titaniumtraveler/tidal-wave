use crate::usb_device::{Color, DeviceConfiguration, LowcutFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct UiState {
    pub cached: DeviceConfiguration,
    pub io: Line,
}

impl UiState {
    pub fn update_device_info(&mut self, config: DeviceConfiguration) -> Line {
        self.cached = config;
        let Line {
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
            err,
        } = &mut self.io;

        Line {
            gain: match gain {
                Some(gain) if config.gain != *gain => {
                    *gain = config.gain;
                    Some(config.gain)
                }
                None => {
                    *gain = Some(config.gain);
                    Some(config.gain)
                }
                _ => None,
            },
            mute: match mute {
                Some(mute) if config.mute != *mute => {
                    *mute = config.mute;
                    Some(config.mute)
                }
                None => {
                    *mute = Some(config.mute);
                    Some(config.mute)
                }
                _ => None,
            },
            clipguard: match clipguard {
                Some(clipguard) if config.clipguard != *clipguard => {
                    *clipguard = config.clipguard;
                    Some(config.clipguard)
                }
                None => {
                    *clipguard = Some(config.clipguard);
                    Some(config.clipguard)
                }
                _ => None,
            },
            phantom: match phantom {
                Some(phantom) if config.phantom != *phantom => {
                    *phantom = config.phantom;
                    Some(config.phantom)
                }
                None => {
                    *phantom = Some(config.phantom);
                    Some(config.phantom)
                }
                _ => None,
            },
            lowcut: match lowcut {
                Some(lowcut) if config.lowcut != *lowcut => {
                    *lowcut = config.lowcut;
                    Some(config.lowcut)
                }
                None => {
                    *lowcut = Some(config.lowcut);
                    Some(config.lowcut)
                }
                _ => None,
            },
            volume: match volume {
                Some(volume) if config.volume != *volume => {
                    *volume = config.volume;
                    Some(config.volume)
                }
                None => {
                    *volume = Some(config.volume);
                    Some(config.volume)
                }
                _ => None,
            },
            mix: match mix {
                Some(mix) if config.mix != *mix => {
                    *mix = config.mix;
                    Some(config.mix)
                }
                None => {
                    *mix = Some(config.mix);
                    Some(config.mix)
                }
                _ => None,
            },
            color_mute: match color_mute {
                Some(color_mute) if config.color_mute != *color_mute => {
                    *color_mute = config.color_mute;
                    Some(config.color_mute)
                }
                None => {
                    *color_mute = Some(config.color_mute);
                    Some(config.color_mute)
                }
                _ => None,
            },
            color_gen: match color_gen {
                Some(color_gen) if config.color_gen != *color_gen => {
                    *color_gen = config.color_gen;
                    Some(config.color_gen)
                }
                None => {
                    *color_gen = Some(config.color_gen);
                    Some(config.color_gen)
                }
                _ => None,
            },
            gain_lock: match gain_lock {
                Some(gain_lock) if config.gain_lock != *gain_lock => {
                    *gain_lock = config.gain_lock;
                    Some(config.gain_lock)
                }
                None => {
                    *gain_lock = Some(config.gain_lock);
                    Some(config.gain_lock)
                }
                _ => None,
            },
            color_gain_reduction: match color_gain_reduction {
                Some(color_gain_reduction)
                    if config.color_gain_reduction != *color_gain_reduction =>
                {
                    *color_gain_reduction = config.color_gain_reduction;
                    Some(config.color_gain_reduction)
                }
                None => {
                    *color_gain_reduction = Some(config.color_gain_reduction);
                    Some(config.color_gain_reduction)
                }
                _ => None,
            },
            clipguard_indicator: match clipguard_indicator {
                Some(clipguard_indicator) if config.clipguard_indicator != *clipguard_indicator => {
                    *clipguard_indicator = config.clipguard_indicator;
                    Some(config.clipguard_indicator)
                }
                None => {
                    *clipguard_indicator = Some(config.clipguard_indicator);
                    Some(config.clipguard_indicator)
                }
                _ => None,
            },
            lim: match lim {
                Some(lim) if config.lim != *lim => {
                    *lim = config.lim;
                    Some(config.lim)
                }
                None => {
                    *lim = Some(config.lim);
                    Some(config.lim)
                }
                _ => None,
            },
            persistent: None,
            use_cached: None,
            err: err.take(),
        }
    }

    pub fn update_state(&mut self, line: Line) -> DeviceConfiguration {
        self.cached.merge(&line);
        self.cached
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Line {
    /// Input Gain
    ///
    /// Input Gain in dB. Range 0dB to 75dB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain: Option<u16>,

    // Mute
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,

    /// Clipguard
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clipguard: Option<bool>,

    /// Phantom Power - 48V Phantom Power
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phantom: Option<bool>,

    /// Lowcut Filter
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lowcut: Option<LowcutFilter>,

    /// Monitor Volume
    ///
    /// Monitor volume in dB. Range 0dB to -128dB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<i16>,

    /// Monitor Mix
    ///
    /// Mix between microphone and PC audio in %
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mix: Option<u8>,

    /// Mute Color
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_mute: Option<Color>,

    /// General Color
    ///
    /// For some reason they appear *trice as part of the config bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_gen: Option<Color>,

    /// Wave Gain Lock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain_lock: Option<bool>,

    /// Gain Reduction Color
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_gain_reduction: Option<Color>,

    /// Clipguard Indicator
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clipguard_indicator: Option<bool>,

    /// Low Impedence Mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lim: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none", skip_serializing)]
    pub persistent: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none", skip_serializing)]
    pub use_cached: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none", skip_deserializing)]
    pub err: Option<String>,
}

impl Line {
    pub fn is_empty(&self) -> bool {
        let Line {
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
            err,
            persistent: _,
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
            && err.is_none()
    }
}
