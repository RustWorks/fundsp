//! Render a sequence and save it to disk.

#![allow(unused_must_use)]
#![allow(clippy::precedence)]

use fundsp::hacker::*;
use fundsp::sound::*;

fn main() {
    let mut rng = AttoRand::new(0);

    let bpm = 128.0;
    let bps = bpm / 60.0;

    /*
    let wind = |seed: i64, panning| {
        (noise() | lfo(move |t| xerp11(50.0, 5000.0, fractal_noise(seed, 6, 0.5, t * 0.2))))
            >> bandpass_q(5.0)
            >> pan(panning)
    };
    */

    let sample_rate = 44100.0;
    // 'x' indicates a drum hit, while '.' is a rest.
    let bassd_line = "x.....x.x.......x.....x.xx......x.....x.x.......x.......x.x.....";
    let snare_line = "....x.......x.......x.......x.......x.......x.......x.......x...";

    /*
    let bd = |seed: i64| {
        bus::<U40, _, _>(|i| {
            let f = xerp(50.0, 2000.0, rnd(i ^ seed));
            lfo(move |t| xerp(f, f * semitone_ratio(-5.0), t))
                >> sine()
                    * lfo(move |t| {
                        xerp(1.0, 0.02, dexerp(50.0, 2000.0, f)) * exp(-t * f * f * 0.002)
                    })
                >> pan(0.0)
        })
    };

    let bd2 = || {
        let sweep = (lfo(|t| xerp(100.0, 50.0, t)) >> saw() | lfo(|t| xerp(3000.0, 3.0, t)))
            >> !lowpass_q(2.0)
            >> lowpass_q(1.0);
        sweep >> pinkpass() >> shape(Shape::Tanh(2.0)) >> pan(0.0)
    };
    */

    let stab = move || {
        fundsp::sound::pebbles(14.0, 200)
            * lfo(move |t| {
                if t * bps - round(t * bps) > 0.0 && round(t * bps) < 32.0 {
                    0.1
                } else {
                    0.0
                }
            })
            >> highpass_hz(3200.0, 1.0)
            >> phaser(0.85, |t| sin_hz(0.1, t) * 0.5 + 0.5)
            >> pan(0.0)
    };

    let mut sequencer = Sequencer64::new(sample_rate, 2);

    sequencer.add(0.0, 60.0, 0.0, 0.0, Box::new(stab() * 0.4));

    let length = bassd_line.as_bytes().len();
    let duration = length as f64 / bpm_hz(bpm) / 4.0 * 2.0 + 1.0;

    for i in 0..length * 2 {
        let t0 = i as f64 / bpm_hz(bpm) / 4.0;
        let t1 = t0 + 1.0;
        if bassd_line.as_bytes()[i % length] == b'x' {
            sequencer.add(
                t0,
                t1,
                0.0,
                0.25,
                Box::new(bassdrum(0.2 + rng.get01::<f64>() * 0.02, 180.0, 60.0) >> pan(0.0)),
            );
        }
        if snare_line.as_bytes()[i % length] == b'x' {
            sequencer.add(
                t0,
                t1,
                0.0,
                0.25,
                Box::new(snaredrum(rng.get() as i64, 0.3 + rng.get01::<f64>() * 0.02) >> pan(0.0)),
            );
        }
    }

    let wave = Wave64::render(sample_rate, duration, &mut sequencer);

    let wave = wave.filter(
        duration,
        &mut (multipass() & 0.15 * reverb_stereo(10.0, 1.0)),
    );

    let wave = wave.filter_latency(duration, &mut (limiter_stereo((5.0, 5.0))));

    wave.save_wav16(std::path::Path::new("sequence.wav"));
}
