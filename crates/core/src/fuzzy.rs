use crate::model::ProcessSnapshot;

pub struct FuzzyMatcher(nucleo::Matcher);

impl FuzzyMatcher {
	pub fn new() -> Self {
		Self(nucleo::Matcher::new(nucleo::Config::DEFAULT))
	}

	#[hotpath::measure]
	pub fn matches(&mut self, needle: &str, haystack: &str) -> bool {
		nucleo_fuzzy_score(needle, haystack, &mut self.0).is_some()
	}

	#[hotpath::measure]
	pub fn best_score(
		&mut self,
		needle: &str,
		p: &ProcessSnapshot,
	) -> (u32, u32) {
		best_fuzzy_score(needle, p, &mut self.0)
	}
}

pub(crate) fn nucleo_fuzzy_score(
	needle: &str,
	haystack: &str,
	matcher: &mut nucleo::Matcher,
) -> Option<u32> {
	let pattern = nucleo::pattern::Pattern::new(
		needle,
		nucleo::pattern::CaseMatching::Ignore,
		nucleo::pattern::Normalization::Smart,
		nucleo::pattern::AtomKind::Fuzzy,
	);
	let hs = nucleo::Utf32String::from(haystack);
	pattern.score(hs.slice(..), matcher)
}

pub(crate) fn best_fuzzy_score(
	needle: &str,
	p: &ProcessSnapshot,
	matcher: &mut nucleo::Matcher,
) -> (u32, u32) {
	// Name-first ranking: the displayed name (electron_app_name or name)
	// is what the user sees, so a name match outranks a command-only match.
	let name = p.electron_app_name.as_deref().unwrap_or(&p.name);
	let name_score =
		nucleo_fuzzy_score(needle, &name.to_lowercase(), matcher)
			.unwrap_or(0);
	let cmd_score =
		nucleo_fuzzy_score(needle, &p.command.to_lowercase(), matcher)
			.unwrap_or(0);
	(name_score, cmd_score)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::VramUsage;

	fn matcher() -> FuzzyMatcher {
		FuzzyMatcher::new()
	}

	#[test]
	fn test_fuzzy_match_exact() {
		let mut m = matcher();
		assert!(m.matches("bash", "bash"));
	}

	#[test]
	fn test_fuzzy_match_substring() {
		let mut m = matcher();
		assert!(m.matches("ba", "bash"));
		assert!(m.matches("sh", "bash"));
	}

	#[test]
	fn test_fuzzy_match_case_insensitive() {
		let mut m = matcher();
		assert!(m.matches("BASH", "bash"));
		assert!(m.matches("bash", "BASH"));
	}

	#[test]
	fn test_fuzzy_match_no_match() {
		let mut m = matcher();
		assert!(!m.matches("xyz", "bash"));
	}

	#[test]
	fn test_fuzzy_match_empty_needle() {
		let mut m = matcher();
		assert!(m.matches("", "bash"));
	}

	#[test]
	fn test_nucleo_fuzzy_score_exact() {
		let mut m = matcher();
		let score = nucleo_fuzzy_score("firefox", "firefox", &mut m.0);
		assert!(score.is_some());
	}

	#[test]
	fn test_nucleo_fuzzy_score_no_match() {
		let mut m = matcher();
		let score = nucleo_fuzzy_score("xyzabc", "firefox", &mut m.0);
		assert_eq!(score, None);
	}

	#[test]
	fn test_nucleo_fuzzy_score_empty_needle() {
		let mut m = matcher();
		let score = nucleo_fuzzy_score("", "firefox", &mut m.0);
		assert!(score.is_some());
	}

	#[test]
	fn test_nucleo_fuzzy_score_empty_haystack() {
		let mut m = matcher();
		let score = nucleo_fuzzy_score("test", "", &mut m.0);
		assert_eq!(score, None);
	}

	#[test]
	fn test_best_fuzzy_score_returns_highest() {
		let p = ProcessSnapshot {
			pid: 1,
			ppid: 0,
			name: "firefox".into(),
			user: String::new(),
			state: 'S',
			command: "/usr/lib/firefox/firefox".into(),
			cgroup: String::new(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram: VramUsage::default(),
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: true,
			is_kthread: false,
			is_owned_by_current_user: true,
			is_electron: false,
			electron_app_name: None,
			icon_name: None,
			has_children: false,
		};
		let mut m = matcher();
		let s = m.best_score("fire", &p);
		assert!(s.0 > 0);
	}

	#[test]
	fn test_best_fuzzy_score_no_match_is_zero() {
		let p = ProcessSnapshot {
			pid: 1,
			ppid: 0,
			name: "bash".into(),
			user: String::new(),
			state: 'S',
			command: "/bin/bash".into(),
			cgroup: String::new(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram: VramUsage::default(),
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: true,
			is_electron: false,
			electron_app_name: None,
			icon_name: None,
			has_children: false,
		};
		let mut m = matcher();
		let s = m.best_score("firefox", &p);
		assert_eq!(s, (0, 0));
	}
}
