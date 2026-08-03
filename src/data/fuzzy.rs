use crate::data::model::ProcessInfo;

pub fn fuzzy_match(
	needle: &str,
	haystack: &str,
	matcher: &mut nucleo::Matcher,
) -> bool {
	nucleo_fuzzy_score(needle, haystack, matcher).is_some()
}

pub fn nucleo_fuzzy_score(
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_fuzzy_match_exact() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		assert!(fuzzy_match("bash", "bash", &mut m));
	}

	#[test]
	fn test_fuzzy_match_substring() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		assert!(fuzzy_match("ba", "bash", &mut m));
		assert!(fuzzy_match("sh", "bash", &mut m));
	}

	#[test]
	fn test_fuzzy_match_case_insensitive() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		assert!(fuzzy_match("BASH", "bash", &mut m));
		assert!(fuzzy_match("bash", "BASH", &mut m));
	}

	#[test]
	fn test_fuzzy_match_no_match() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		assert!(!fuzzy_match("xyz", "bash", &mut m));
	}

	#[test]
	fn test_fuzzy_match_empty_needle() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let result = fuzzy_match("", "bash", &mut m);
		assert!(result);
	}

	#[test]
	fn test_nucleo_fuzzy_score_exact() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let score = nucleo_fuzzy_score("firefox", "firefox", &mut m);
		assert!(score.is_some());
	}

	#[test]
	fn test_nucleo_fuzzy_score_no_match() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let score = nucleo_fuzzy_score("xyzabc", "firefox", &mut m);
		assert_eq!(score, None);
	}

	#[test]
	fn test_nucleo_fuzzy_score_empty_needle() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let score = nucleo_fuzzy_score("", "firefox", &mut m);
		assert!(score.is_some());
	}

	#[test]
	fn test_nucleo_fuzzy_score_empty_haystack() {
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let score = nucleo_fuzzy_score("test", "", &mut m);
		assert_eq!(score, None);
	}

	#[test]
	fn test_best_fuzzy_score_returns_highest() {
		let p = ProcessInfo {
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
			vram_bytes: None,
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
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let s = best_fuzzy_score("fire", &p, &mut m);
		assert!(s > 0);
	}

	#[test]
	fn test_best_fuzzy_score_no_match_is_zero() {
		let p = ProcessInfo {
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
			vram_bytes: None,
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
		let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
		let s = best_fuzzy_score("firefox", &p, &mut m);
		assert_eq!(s, 0);
	}
}

#[hotpath::measure]
pub fn best_fuzzy_score(
	needle: &str,
	p: &ProcessInfo,
	matcher: &mut nucleo::Matcher,
) -> u32 {
	let name_score =
		nucleo_fuzzy_score(needle, &p.name.to_lowercase(), matcher)
			.unwrap_or(0);
	let cmd_score =
		nucleo_fuzzy_score(needle, &p.command.to_lowercase(), matcher)
			.unwrap_or(0);
	let electron_score = nucleo_fuzzy_score(
		needle,
		&p.electron_app_name
			.as_deref()
			.unwrap_or_default()
			.to_lowercase(),
		matcher,
	)
	.unwrap_or(0);
	name_score.max(cmd_score).max(electron_score)
}
