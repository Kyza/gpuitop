use std::collections::HashSet;
use std::io::Write;
use std::{env, fs};

fn main() {
	let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();

	let root = metadata.root_package().unwrap();
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
		r#"#[derive(Clone)]
pub struct DepInfo {{
	pub name: &'static str,
	pub version: &'static str,
	pub license: &'static str,
	pub repository: &'static str,
	pub homepage: &'static str,
	pub description: &'static str,
}}

pub const DIRECT_DEPS: &[DepInfo] = &[{}];
"#,
		deps.iter()
			.map(|p| {
				format!(
					r#"DepInfo {{
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
}
