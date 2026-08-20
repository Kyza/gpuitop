#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "windows")]
pub use windows::*;

pub fn cpu_count() -> usize {
	#[cfg(any(target_os = "linux", target_os = "macos"))]
	{
		let n = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) };
		if n < 1 {
			1
		} else {
			n as usize
		}
	}
	#[cfg(not(any(target_os = "linux", target_os = "macos")))]
	{
		1
	}
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
	use super::*;

	struct AffinityGuard(libc::cpu_set_t);

	impl AffinityGuard {
		fn capture() -> Self {
			let mut saved: libc::cpu_set_t = unsafe { std::mem::zeroed() };
			let ret = unsafe {
				libc::sched_getaffinity(
					0,
					std::mem::size_of::<libc::cpu_set_t>(),
					&mut saved,
				)
			};
			assert_eq!(ret, 0, "sched_getaffinity failed");
			AffinityGuard(saved)
		}
	}

	impl Drop for AffinityGuard {
		fn drop(&mut self) {
			unsafe {
				libc::sched_setaffinity(
					0,
					std::mem::size_of::<libc::cpu_set_t>(),
					&self.0,
				);
			}
		}
	}

	#[test]
	fn cpu_count_tracks_all_configured_cpus_not_calling_thread_affinity() {
		let _guard = AffinityGuard::capture();
		let total =
			unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
		assert!(total >= 1);
		if total < 2 {
			return;
		}
		assert_eq!(cpu_count(), total);

		let mut one: libc::cpu_set_t = unsafe { std::mem::zeroed() };
		unsafe {
			libc::CPU_ZERO(&mut one);
			libc::CPU_SET(0, &mut one);
		}
		let ret = unsafe {
			libc::sched_setaffinity(
				0,
				std::mem::size_of::<libc::cpu_set_t>(),
				&one,
			)
		};
		assert_eq!(ret, 0, "sched_setaffinity to cpu 0 failed");

		assert_eq!(
			cpu_count(),
			total,
			"cpu_count must ignore the calling thread's affinity"
		);
	}
}
