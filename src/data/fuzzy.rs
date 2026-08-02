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
