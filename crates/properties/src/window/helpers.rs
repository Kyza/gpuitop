// Pure helpers for the properties window body, extracted so they're testable
// without pulling in the gpui element tree.

pub fn quote_command(command: &[String]) -> String {
	command
		.iter()
		.map(|s| {
			if s.contains(char::is_whitespace) {
				format!("'{}'", s.replace('\'', "\\'"))
			} else {
				s.clone()
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

pub fn filter_env_vars<'a>(
	vars: &'a [(String, String)],
	name_search: &str,
	content_search: &str,
) -> Vec<(&'a str, &'a str)> {
	let name_lower = name_search.to_lowercase();
	let content_lower = content_search.to_lowercase();
	let mut filtered: Vec<(&str, &str)> = vars
		.iter()
		.map(|(k, v)| (k.as_str(), v.as_str()))
		.filter(|(k, v)| {
			let name_match = name_search.is_empty()
				|| k.to_lowercase().contains(&name_lower);
			let content_match = content_search.is_empty()
				|| v.to_lowercase().contains(&content_lower);
			name_match && content_match
		})
		.collect();
	filtered.sort_by(|a, b| a.0.cmp(b.0));
	filtered
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn quote_command_quotes_whitespace_and_escapes() {
		let cmd = vec![
			"/usr/bin/prog".to_string(),
			"--flag value".to_string(),
			"it's".to_string(),
			"flag it's".to_string(),
		];
		assert_eq!(
			quote_command(&cmd),
			"/usr/bin/prog '--flag value' it's 'flag it\\'s'"
		);
	}

	#[test]
	fn quote_command_plain_passthrough() {
		let cmd = vec!["a".to_string(), "b".to_string()];
		assert_eq!(quote_command(&cmd), "a b");
	}

	#[test]
	fn filter_env_vars_matches_name_and_content_and_sorts() {
		let vars = vec![
			("ZZZ".to_string(), "1".to_string()),
			("AAA".to_string(), "hello world".to_string()),
			("BBB".to_string(), "hi".to_string()),
		];
		let out = filter_env_vars(&vars, "aa", "");
		assert_eq!(
			out.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
			vec!["AAA"]
		);

		let out = filter_env_vars(&vars, "", "hello");
		assert_eq!(
			out.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
			vec!["AAA"]
		);

		let out = filter_env_vars(&vars, "", "");
		assert_eq!(
			out.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
			vec!["AAA", "BBB", "ZZZ"]
		);
	}

	#[test]
	fn filter_env_vars_empty_search_returns_all_sorted() {
		let vars = vec![
			("B".to_string(), "2".to_string()),
			("a".to_string(), "1".to_string()),
		];
		let out = filter_env_vars(&vars, "", "");
		// Byte-wise sort (uppercase before lowercase).
		assert_eq!(
			out.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
			vec!["B", "a"]
		);
	}
}
