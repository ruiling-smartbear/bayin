//! Wave steepening: why brass and horns sound brassy when loud. Sound travels a little faster
//! where the pressure is high, so over the length of a narrow tube the crests of a loud wave
//! gain on its troughs and the wave leans toward a sawtooth, with strong upper harmonics. The
//! airway network is linear in this respect, so this stage does it at the tube's outlet: each
//! instant of the wave arrives early or late in proportion to its own pressure.
//!
//! The pressure is taken relative to the wave's recent peak, and `amount` (a fraction of the
//! note's period at that peak) is the caller's: it should grow with how hard the tube is blown.

pub struct Steepen {
    buf: Vec<f64>,
    pos: usize,
    /// the middle of the line (samples): the delay of a sample at zero pressure
    mid: usize,
    /// the wave's recent peak, and its decay per sample
    peak: f64,
    decay: f64,
}

impl Steepen {
    /// `max_shift`: the furthest a sample can be moved (samples).
    pub fn new(fs: f64, max_shift: f64) -> Steepen {
        let mid = max_shift.ceil() as usize + 2;
        Steepen { buf: vec![0.0; 2 * mid + 2], pos: 0, mid, peak: 0.0, decay: 1.0 - 1.0 / (0.15 * fs) }
    }

    /// `shift`: how many samples early the wave's peak arrives (0 leaves the wave as it is).
    #[inline]
    pub fn next(&mut self, x: f64, shift: f64) -> f64 {
        let n = self.buf.len();
        self.buf[self.pos] = x;
        let centre = self.buf[(self.pos + n - self.mid) % n];
        self.peak = (self.peak * self.decay).max(centre.abs());
        let rel = if self.peak > 0.0 { centre / self.peak } else { 0.0 };
        let lim = (self.mid - 2) as f64;
        let d = (shift * rel).clamp(-lim, lim);
        // high pressure arrives early: read a sample written after the centre one
        let at = (self.pos + n - self.mid) as f64 + d;
        let i = at.floor();
        let frac = at - i;
        let a = self.buf[(i as usize) % n];
        let b = self.buf[(i as usize + 1) % n];
        self.pos = (self.pos + 1) % n;
        a + (b - a) * frac
    }
}
