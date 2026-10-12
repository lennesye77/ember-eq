use nih_plug::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

mod dsp;
mod editor;

use dsp::{Biquad, BiquadKind};

pub struct EmberEq {
    params: Arc<EmberParams>,
    sample_rate: f32,
    left: [Biquad; 6],
    right: [Biquad; 6],
    display_sample_rate: Arc<AtomicU32>,
}

#[derive(Params)]
pub struct EmberParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<nih_plug_egui::EguiState>,

    #[id = "bypass"]
    pub bypass: BoolParam,

    #[id = "out_gain"]
    pub output_gain: FloatParam,

    #[nested(array, group = "Bands")]
    pub bands: [BandParams; 6],
}

#[derive(Params)]
pub struct BandParams {
    #[id = "enabled"]
    pub enabled: BoolParam,

    #[id = "freq"]
    pub frequency: FloatParam,

    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "q"]
    pub q: FloatParam,

    #[id = "kind"]
    pub kind: EnumParam<FilterKind>,
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    #[name = "Bell"]
    Bell,
    #[name = "Low Shelf"]
    LowShelf,
    #[name = "High Shelf"]
    HighShelf,
    #[name = "Low Cut"]
    LowCut,
    #[name = "High Cut"]
    HighCut,
}

impl Default for EmberParams {
    fn default() -> Self {
        let frequencies = [55.0, 140.0, 500.0, 1800.0, 6500.0, 14000.0];

        Self {
            editor_state: nih_plug_egui::EguiState::from_size(980, 720),
            bypass: BoolParam::new("Bypass", false),
            output_gain: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear {
                    min: -18.0,
                    max: 18.0,
                },
            )
            .with_unit(" dB")
            .with_smoother(SmoothingStyle::Linear(20.0)),
            bands: std::array::from_fn(|i| {
                let default_kind = match i {
                    0 => FilterKind::LowCut,
                    1 => FilterKind::LowShelf,
                    4 => FilterKind::HighShelf,
                    5 => FilterKind::HighCut,
                    _ => FilterKind::Bell,
                };

                BandParams {
                    enabled: BoolParam::new(format!("Band {} On", i + 1), true),
                    frequency: FloatParam::new(
                        format!("Band {} Frequency", i + 1),
                        frequencies[i],
                        FloatRange::Skewed {
                            min: 20.0,
                            max: 20_000.0,
                            factor: FloatRange::skew_factor(-2.3),
                        },
                    )
                    .with_unit(" Hz"),
                    gain: FloatParam::new(
                        format!("Band {} Gain", i + 1),
                        0.0,
                        FloatRange::Linear {
                            min: -18.0,
                            max: 18.0,
                        },
                    )
                    .with_unit(" dB"),
                    q: FloatParam::new(
                        format!("Band {} Q", i + 1),
                        0.707,
                        FloatRange::Skewed {
                            min: 0.15,
                            max: 12.0,
                            factor: FloatRange::skew_factor(-1.5),
                        },
                    ),
                    kind: EnumParam::new(format!("Band {} Type", i + 1), default_kind),
                }
            }),
        }
    }
}

impl Default for EmberEq {
    fn default() -> Self {
        Self {
            params: Arc::new(EmberParams::default()),
            sample_rate: 44_100.0,
            left: std::array::from_fn(|_| Biquad::default()),
            right: std::array::from_fn(|_| Biquad::default()),
            display_sample_rate: Arc::new(AtomicU32::new(44_100.0_f32.to_bits())),
        }
    }
}

impl EmberEq {
    fn update_filters(&mut self) {
        for i in 0..6 {
            let band = &self.params.bands[i];
            let kind = match band.kind.value() {
                FilterKind::Bell => BiquadKind::Peak,
                FilterKind::LowShelf => BiquadKind::LowShelf,
                FilterKind::HighShelf => BiquadKind::HighShelf,
                FilterKind::LowCut => BiquadKind::HighPass,
                FilterKind::HighCut => BiquadKind::LowPass,
            };

            let coefficients = Biquad::coefficients(
                kind,
                self.sample_rate,
                band.frequency.value(),
                band.gain.value(),
                band.q.value(),
            );

            self.left[i].set_coefficients(coefficients);
            self.right[i].set_coefficients(coefficients);
        }
    }
}

impl Plugin for EmberEq {
    const NAME: &'static str = "Ember EQ";
    const VENDOR: &'static str = "Cove Labs";
    const URL: &'static str = "https://github.com/lennesye77/ember-eq";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.display_sample_rate.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.display_sample_rate
            .store(self.sample_rate.to_bits(), Ordering::Relaxed);
        for filter in self.left.iter_mut().chain(self.right.iter_mut()) {
            filter.reset();
        }
        self.update_filters();
        true
    }

    fn reset(&mut self) {
        for filter in self.left.iter_mut().chain(self.right.iter_mut()) {
            filter.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.params.bypass.value() {
            return ProcessStatus::Normal;
        }

        self.update_filters();

        for mut frame in buffer.iter_samples() {
            let output_gain = util::db_to_gain(self.params.output_gain.smoothed.next());

            for (channel_index, sample) in frame.iter_mut().enumerate() {
                let bank = if channel_index == 0 {
                    &mut self.left
                } else {
                    &mut self.right
                };

                let mut value = *sample;
                for (band_index, filter) in bank.iter_mut().enumerate() {
                    if self.params.bands[band_index].enabled.value() {
                        value = filter.process(value);
                    }
                }

                *sample = value * output_gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for EmberEq {
    const CLAP_ID: &'static str = "com.covelabs.ember-eq";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A warm six-band parametric EQ from Cove Labs.");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Equalizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for EmberEq {
    const VST3_CLASS_ID: [u8; 16] = *b"CoveLabsEmberEQ1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(EmberEq);
nih_export_vst3!(EmberEq);
