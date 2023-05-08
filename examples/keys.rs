//! Play notes interactively on a virtual keyboard.
//! Please run me in release mode!
#![allow(clippy::precedence)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use eframe::egui;
use egui::*;
use fundsp::hacker::*;

#[allow(dead_code)]
struct State {
    id: Vec<Option<EventId>>,
    sequencer: Sequencer64,
    net: Net64,
}

static KEYS: [Key; 24] = [
    Key::Z,
    Key::S,
    Key::X,
    Key::D,
    Key::C,
    Key::V,
    Key::G,
    Key::B,
    Key::H,
    Key::N,
    Key::J,
    Key::M,
    Key::Q,
    Key::Num2,
    Key::W,
    Key::Num3,
    Key::E,
    Key::R,
    Key::Num5,
    Key::T,
    Key::Num6,
    Key::Y,
    Key::Num7,
    Key::U,
];

fn main() {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()).unwrap(),
        _ => panic!("Unsupported format"),
    }
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f64>,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut sequencer = Sequencer64::new(false, 1);
    let sequencer_backend = sequencer.backend();

    let mut net = Net64::wrap(Box::new(sequencer_backend));
    net = net >> pan(0.0);
    net = net >> (chorus(0, 0.0, 0.01, 0.2) | chorus(1, 0.0, 0.01, 0.2));
    net = net >> (multipass() & 0.2 * reverb_stereo(20.0, 2.0));

    net.set_sample_rate(sample_rate);

    // Use block processing for maximum efficiency.
    let mut backend = BlockRateAdapter64::new(Box::new(net.backend()));

    let mut next_value = move || backend.get_stereo();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    let options = eframe::NativeOptions::default();

    let mut state: State = State {
        id: Vec::new(),
        sequencer,
        net,
    };
    state.id.resize(KEYS.len(), None);

    eframe::run_native(
        "Virtual Keyboard Example",
        options,
        Box::new(|_cc| Box::new(state)),
    )
    .unwrap();

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: SizedSample + FromSample<f64>,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left: T = T::from_sample(sample.0);
        let right: T = T::from_sample(sample.1);

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}

impl eframe::App for State {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Virtual Keyboard Example");

            #[allow(clippy::needless_range_loop)]
            for i in 0..KEYS.len() {
                if ctx.input(|c| !c.key_down(KEYS[i])) {
                    if let Some(id) = self.id[i] {
                        // Start fading out existing note.
                        self.sequencer.edit_relative(id, 0.2, 0.2);
                        self.id[i] = None;
                    }
                }
                if ctx.input(|c| c.key_down(KEYS[i])) && self.id[i].is_none() {
                    // Insert new note. We set the end time to infinity initially.
                    self.id[i] = Some(self.sequencer.push_relative(
                        0.0,
                        f64::INFINITY,
                        Fade::Smooth,
                        0.02,
                        0.2,
                        Box::new(saw_hz(midi_hz(40.0 + i as f64))),
                    ));
                }
            }
        });
    }
}