use procfs::process::{Process, Stat, Status};

use crate::ReadError;

// The shared per-PID /proc read: stat + status + cmdline read once. Both the
// tick collector and the properties window consume it, so identity fields
// (name, cpu ticks, hertz conversion) come from one place and the two views
// of a process can't disagree. environ stays out — the tick collector reads
// it on demand (kthread skip + per-PID GUI cache) — and single-consumer
// fields (exe, limits, smaps, …) stay consumer-side.
pub struct ProcBasics {
	pub stat: Stat,
	pub status: Status,
	pub cmdline: Vec<String>,
	pub is_kthread: bool,
}

impl ProcBasics {
	#[hotpath::measure]
	pub fn read(p: &Process) -> Result<ProcBasics, ReadError> {
		let stat = p.stat().map_err(map_err)?;
		let status = p.status().map_err(map_err)?;
		// Kernel threads have no cmdline (always empty); skip the read.
		let is_kthread = stat.ppid == 2 || stat.comm.starts_with('[');
		let cmdline = if is_kthread {
			Vec::new()
		} else {
			p.cmdline().unwrap_or_default()
		};
		Ok(ProcBasics {
			stat,
			status,
			cmdline,
			is_kthread,
		})
	}

	// User + system + children user + children system CPU ticks.
	pub fn cpu_tick_sum(&self) -> u64 {
		self.stat.utime
			+ self.stat.stime
			+ self.stat.cutime as u64
			+ self.stat.cstime as u64
	}

	pub fn cpu_time_secs(&self) -> f64 {
		self.cpu_tick_sum() as f64 / procfs::ticks_per_second() as f64
	}

	// The argv0 basename when a cmdline exists, falling back to comm — the
	// same name the process table shows.
	pub fn name(&self) -> String {
		self.cmdline
			.first()
			.filter(|a| !a.is_empty())
			.and_then(|argv0| {
				std::path::Path::new(argv0)
					.file_name()
					.and_then(|n| n.to_str())
			})
			.map(|s| s.to_string())
			.unwrap_or_else(|| self.stat.comm.clone())
	}
}

pub fn username_for_uid(uid: u32) -> String {
	users::get_user_by_uid(uid)
		.map(|u| u.name().to_string_lossy().into_owned())
		.unwrap_or_else(|| uid.to_string())
}

fn map_err(e: procfs::ProcError) -> ReadError {
	match e {
		procfs::ProcError::NotFound(_) => ReadError::Dead,
		procfs::ProcError::Io(err, _)
			if err.kind() == std::io::ErrorKind::NotFound =>
		{
			ReadError::Dead
		}
		_ => ReadError::Unavailable,
	}
}
