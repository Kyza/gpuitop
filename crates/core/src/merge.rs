use ron2::Value;

// `deep_merge` runs on `ron2::Value`, not `ron::Value`: ron::Value is lossy
// for this round-trip and would corrupt the merged config. It collapses named
// struct fields into an unordered `Map` (re-serializing `(a: 1)` as
// `{ "a": 1 }`), turns tuples into sequences (`(1100, 700)` -> `[1100, 700]`),
// and drops bare unit-variant names (`Auto` -> `()`). Any of those breaks
// re-deserializing the result into `Config`. `ron2::Value` keeps the
// `Struct` / `Tuple` / `Named` variants, so the config's shape survives (see
// `test_ron2_preserves_shape_but_ron_is_lossy`).
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

	#[test]
	fn test_ron2_preserves_shape_but_ron_is_lossy() {
		let input = "(window_size: (1100, 700), gpu_data: On)";

		// ron2 keeps the tuple and the bare unit-variant name, so the merged
		// config still deserializes.
		let r2: ron2::Value = input.parse().unwrap();
		match &r2 {
			ron2::Value::Struct(fields) => {
				assert!(fields.iter().any(|(k, v)| {
					k == "window_size" && matches!(v, ron2::Value::Tuple(_))
				}));
				assert!(fields.iter().any(|(k, v)| {
					k == "gpu_data"
						&& matches!(v, ron2::Value::Named { name, .. }
							if name == "On")
				}));
			}
			other => panic!("expected Struct, got {other:?}"),
		}

		// ron::Value collapses the tuple into a seq and the variant `Auto`
		// into `()`, so it cannot round-trip config RON.
		let r: ron::Value = input.parse().unwrap();
		let out = ron::to_string(&r).unwrap();
		assert_ne!(out, input);
		assert!(out.contains("()"), "bare variant name lost: {out}");
	}
}
