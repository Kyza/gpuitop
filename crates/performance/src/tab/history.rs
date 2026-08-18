pub const HISTORY_LEN: usize = 60;

#[derive(Clone)]
pub struct Sample {
	pub label: String,
	pub cpu: f64,
	pub mem_used: f64,
	pub net_rx: f64,
	pub net_tx: f64,
	pub disk_read: f64,
	pub disk_write: f64,
	pub gpu_utilization: f64,
}

pub struct History {
	samples: Vec<Sample>,
	tick: u64,
}

impl History {
	pub fn new() -> Self {
		Self {
			samples: Vec::new(),
			tick: 0,
		}
	}

	pub fn push(&mut self, mut sample: Sample) {
		self.tick += 1;
		sample.label = self.tick.to_string();
		self.samples.push(sample);
		if self.samples.len() > HISTORY_LEN {
			self.samples.remove(0);
		}
	}

	pub fn samples(&self) -> Vec<Sample> {
		self.samples.clone()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn history_caps_at_history_len() {
		let mut h = History::new();
		for i in 0..(HISTORY_LEN + 10) {
			h.push(Sample {
				label: String::new(),
				cpu: i as f64,
				mem_used: 0.0,
				net_rx: 0.0,
				net_tx: 0.0,
				disk_read: 0.0,
				disk_write: 0.0,
				gpu_utilization: 0.0,
			});
		}
		assert_eq!(h.samples().len(), HISTORY_LEN);
	}

	#[test]
	fn history_labels_are_monotonic_and_unique() {
		let mut h = History::new();
		for i in 0..3 {
			h.push(Sample {
				label: String::new(),
				cpu: i as f64,
				mem_used: 0.0,
				net_rx: 0.0,
				net_tx: 0.0,
				disk_read: 0.0,
				disk_write: 0.0,
				gpu_utilization: 0.0,
			});
		}
		let samples = h.samples();
		let labels: Vec<&str> =
			samples.iter().map(|s| s.label.as_str()).collect();
		assert_eq!(labels, vec!["1", "2", "3"]);
	}

	#[test]
	fn history_evicts_oldest_first() {
		let mut h = History::new();
		for i in 0..(HISTORY_LEN + 1) {
			h.push(Sample {
				label: String::new(),
				cpu: i as f64,
				mem_used: 0.0,
				net_rx: 0.0,
				net_tx: 0.0,
				disk_read: 0.0,
				disk_write: 0.0,
				gpu_utilization: 0.0,
			});
		}
		let samples = h.samples();
		assert_eq!(samples[0].cpu, 1.0);
		assert_eq!(samples.last().unwrap().cpu, HISTORY_LEN as f64);
	}
}
