#[derive(Clone, Debug)]
pub enum Scale {
    Linear(Linear),
    // Log(Log), Time(Time) later
}

#[derive(Clone, Debug)]
pub struct Linear {
    pub domain: (f64, f64),
    pub range: (f64, f64),
}

impl Linear {
    pub fn new(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self { domain, range }
    }

    pub fn map(&self, v: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        if d1 == d0 {
            return r0;
        }
        let t = (v - d0) / (d1 - d0);
        r0 + t * (r1 - r0)
    }
}

impl Scale {
    pub fn linear(domain: (f64, f64), range: (f64, f64)) -> Self {
        Scale::Linear(Linear::new(domain, range))
    }

    pub fn map(&self, v: f64) -> f64 {
        match self {
            Scale::Linear(l) => l.map(v),
        }
    }
}
