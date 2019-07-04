pub struct OutputMetricsTracker {
    sum: f64,
    count: usize,
}

impl OutputMetricsTracker {
    #[inline(always)]
    pub fn new() -> Self {
        OutputMetricsTracker {
            sum: 0.0,
            count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.count = 0;
    }

    pub fn update(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
    }

    pub fn average(&self) -> f64 {
        let c = if self.count > 0 {
            self.count as f64
        } else {
            1.0
        };
        self.sum / c
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        self.count
    }
}
