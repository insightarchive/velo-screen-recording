/// Critically damped spring, integrated implicitly so it stays stable at any frame rate.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub pos: f64,
    pub vel: f64,
}

impl Spring {
    pub const fn new(pos: f64) -> Self {
        Self { pos, vel: 0.0 }
    }

    /// Advance by `dt` seconds toward `target`. `omega` is the natural frequency in rad/s.
    pub fn step(&mut self, target: f64, omega: f64, dt: f64) {
        let f = 1.0 + 2.0 * dt * omega;
        let oo = omega * omega;
        let hoo = dt * oo;
        let hhoo = dt * hoo;
        let det = f + hhoo;
        let pos = (f * self.pos + dt * self.vel + hhoo * target) / det;
        let vel = (self.vel + hoo * (target - self.pos)) / det;
        self.pos = pos;
        self.vel = vel;
    }
}
