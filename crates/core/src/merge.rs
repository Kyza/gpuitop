use ron2::Value;

pub fn deep_merge(base: &mut Value, overlay: &Value) {
	if let Value::Struct(b_vals) = &mut *base {
		if let Value::Struct(o_vals) = overlay {
			for (o_name, o_val) in o_vals.iter() {
				match b_vals
					.iter_mut()
					.find(|(n, _)| n == o_name)
					.map(|(_, v)| v)
				{
					Some(existing) => deep_merge(existing, o_val),
					None => {
						b_vals.push((o_name.clone(), o_val.clone()));
					}
				}
			}
			return;
		}
	}
	*base = overlay.clone();
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse(s: &str) -> Value {
		s.parse().expect("RON parse failed")
	}

	#[test]
	fn test_merge_structs_adds_new_field() {
		let mut base = parse("(a: 1)");
		let overlay = parse("(b: 2)");
		deep_merge(&mut base, &overlay);
		let expected = parse("(a: 1, b: 2)");
		assert_eq!(base, expected);
	}

	#[test]
	fn test_merge_structs_overwrites_field() {
		let mut base = parse("(a: 1, b: 2)");
		let overlay = parse("(b: 3)");
		deep_merge(&mut base, &overlay);
		assert_eq!(base, parse("(a: 1, b: 3)"));
	}

	#[test]
	fn test_merge_nested_structs() {
		let mut base = parse("(x: (a: 1, b: 2))");
		let overlay = parse("(x: (b: 3, c: 4))");
		deep_merge(&mut base, &overlay);
		assert_eq!(base, parse("(x: (a: 1, b: 3, c: 4))"));
	}

	#[test]
	fn test_merge_with_enums() {
		let mut base = parse("(theme: Dark, refresh_ms: 1500)");
		let overlay = parse("(refresh_ms: 500)");
		deep_merge(&mut base, &overlay);
		assert_eq!(base, parse("(theme: Dark, refresh_ms: 500)"));
	}

	#[test]
	fn test_merge_scalar_into_struct() {
		let mut base = parse("(a: 1)");
		let overlay = parse("5");
		deep_merge(&mut base, &overlay);
		assert_eq!(base, parse("5"));
	}
}
