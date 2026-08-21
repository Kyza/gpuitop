use std::collections::HashSet;
use std::io::Write;
use std::{env, fs};

fn main() {
	let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();

	let root = metadata
		.packages
		.iter()
		.find(|p| p.name == env!("CARGO_PKG_NAME"))
		.unwrap();
	let runtime_deps: HashSet<&str> = root
		.dependencies
		.iter()
		.filter(|d| matches!(d.kind, cargo_metadata::DependencyKind::Normal))
		.map(|d| d.name.as_str())
		.collect();

	let mut deps: Vec<_> = metadata
		.packages
		.iter()
		.filter(|p| {
			runtime_deps.contains(p.name.as_str())
				&& (p.license.as_ref().is_some_and(|l| !l.is_empty())
					|| p.repository.is_some())
		})
		.collect();

	deps.sort_by_key(|p| p.name.clone());

	let out_dir = env::var("OUT_DIR").unwrap();
	let dest = std::path::Path::new(&out_dir).join("built.rs");
	let mut f = fs::File::create(&dest).unwrap();

	writeln!(
		f,
		r#"pub const DIRECT_DEPS: &[gpuitop_core::about::DepInfo] = &[{}];
"#,
		deps.iter()
			.map(|p| {
				format!(
					r#"gpuitop_core::about::DepInfo {{
		name: "{name}",
		version: "{version}",
		license: "{license}",
		repository: "{repo}",
		homepage: "{homepage}",
		description: "{desc}",
	}},"#,
					name = p.name,
					version = p.version,
					license = p.license.as_deref().unwrap_or(""),
					repo = p.repository.as_deref().unwrap_or(""),
					homepage = p.homepage.as_deref().unwrap_or(""),
					desc = p
						.description
						.as_deref()
						.unwrap_or("")
						.replace('"', "\\\"")
						.replace('\n', " "),
				)
			})
			.collect::<Vec<_>>()
			.join("\n")
	)
	.unwrap();

	println!("cargo:rerun-if-changed=Cargo.lock");

	if env::var_os("CARGO_FEATURE_PACKAGING").is_some() {
		let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
		if let Some(desktop) = gpuitop_packaging::desktop(&target_os) {
			let target = env::var("CARGO_TARGET_DIR").unwrap();
			let profile = env::var("PROFILE").unwrap();
			let dest_dir = std::path::Path::new(&target).join(&profile);
			fs::write(dest_dir.join("gpuitop.desktop"), desktop).unwrap();
		}
	}
}
