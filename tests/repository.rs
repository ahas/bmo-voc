//! Checks that span the repository: what its packages may depend on, its layout and the README's folder table,
//! the testbed's declaration and the service's unit, and what no tracked file may hold (Hangul in Rust source and the personal data
//! `PERSONAL-DATA.md` names).

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path};
use std::process::Command;
use std::sync::LazyLock;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde_json::Value;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The only `bmo-*` packages a package here may depend on: bmo-chat's console face. Other units are reached
/// through their documented APIs.
const BASE_LIBRARIES: [&str; 1] = ["bmo-console-face"];

/// Shared crates of the single repository this platform grew from; no package here may take one as its name.
const OLD_SHARED_CRATES: [&str; 11] = [
  "archive-pool",
  "archive-read",
  "bmo-lang",
  "bmo-markup",
  "bmo-policy",
  "bmo-tool",
  "room-acts",
  "seoul-clock",
  "harness-providers",
  "agent-host",
  "test-support",
];

/// File names that declare a package or a project to some other build tool.
const OTHER_MANIFESTS: [&str; 10] = [
  "package.json",
  "pyproject.toml",
  "setup.py",
  "go.mod",
  "deno.json",
  "bun.lockb",
  "pnpm-workspace.yaml",
  "Gemfile",
  "pom.xml",
  "build.gradle",
];

/// The Hangul blocks of Unicode: Jamo, compatibility Jamo, Jamo extended A and B, syllables, and the halfwidth
/// forms.
const HANGUL: [(u32, u32); 6] = [
  (0x1100, 0x11FF),
  (0x3130, 0x318F),
  (0xA960, 0xA97F),
  (0xAC00, 0xD7AF),
  (0xD7B0, 0xD7FF),
  (0xFFA0, 0xFFDC),
];

fn metadata() -> Value {
  let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
  let output = Command::new(cargo)
    .args(["metadata", "--no-deps", "--format-version", "1"])
    .current_dir(ROOT)
    .output()
    .expect("cargo metadata runs");
  assert!(
    output.status.success(),
    "cargo metadata failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  serde_json::from_slice(&output.stdout).expect("cargo metadata answers JSON")
}

/// Every file git tracks or would track: the committed ones and the new ones it does not ignore.
fn tracked_files() -> Vec<String> {
  let output = Command::new("git")
    .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
    .current_dir(ROOT)
    .output()
    .expect("git ls-files runs");
  assert!(
    output.status.success(),
    "git ls-files failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  let mut files: Vec<String> = String::from_utf8(output.stdout)
    .expect("tracked paths are UTF-8")
    .split('\0')
    .filter(|path| !path.is_empty())
    .map(str::to_owned)
    .collect();
  files.sort();
  files.dedup();
  files
}

/// The text of a tracked file, or nothing when it is gone from the work tree or is not UTF-8 text.
fn text_of(path: &str) -> Option<String> {
  match fs::read(Path::new(ROOT).join(path)) {
    Ok(bytes) => String::from_utf8(bytes).ok(),
    Err(error) if error.kind() == ErrorKind::NotFound => None,
    Err(error) => panic!("{path} is unreadable: {error}"),
  }
}

fn tracked_texts() -> Vec<(String, String)> {
  tracked_files()
    .into_iter()
    .filter_map(|path| text_of(&path).map(|text| (path, text)))
    .collect()
}

fn is_hangul(character: char) -> bool {
  let point = u32::from(character);
  HANGUL.iter().any(|&(first, last)| (first..=last).contains(&point))
}

/// One form no tracked file may hold, and the kind the check reports it as (`PERSONAL-DATA.md`).
struct PersonalData {
  kind: &'static str,
  pattern: Regex,
}

/// The forms no tracked file may hold, each with the kind the check reports it as. No value is admitted: every
/// form a line holds fails (`PERSONAL-DATA.md`).
fn personal_data() -> &'static [PersonalData] {
  static FORMS: LazyLock<Vec<PersonalData>> = LazyLock::new(|| {
    let form = |kind, source: &str| PersonalData {
      kind,
      pattern: Regex::new(source).expect("a personal-data form"),
    };

    vec![
      form("a mail address", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"),
      form("a home path", r"(?:/home/|/Users/|[A-Za-z]:\\Users\\)[A-Za-z0-9._-]+"),
      form("a telephone number", r"\b01[016789][-. ]?[0-9]{3,4}[-. ][0-9]{4}\b"),
      form("a telephone number", r"\b0[2-9][0-9]?[-. ][0-9]{3,4}[-. ][0-9]{4}\b"),
      form("a telephone number", r"\+\d{1,3}[-. ]\d{2,4}[-. ]\d{3,4}"),
      form("a telephone number", r"\+\d{9,15}\b"),
      form("a token", r"\bgh[pousr]_[A-Za-z0-9]{20,}"),
      form("a token", r"\bgithub_pat_[A-Za-z0-9_]{20,}"),
      form("a token", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
      form("a token", r"\bsk-[A-Za-z0-9_-]{20,}"),
      form("a token", r"\bxox[baprs]-[A-Za-z0-9-]{10,}"),
      form("a token", r"\bAIza[0-9A-Za-z_-]{35}"),
      form("a token", r"\bglpat-[A-Za-z0-9_-]{20,}"),
      form("a token", r"\bnpm_[A-Za-z0-9]{36}\b"),
      form("a token", r"-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----"),
      form("a private network address", r"\b10\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\b"),
      form("a private network address", r"\b192\.168\.[0-9]{1,3}\.[0-9]{1,3}\b"),
      form("a private network address", r"\b172\.(?:1[6-9]|2[0-9]|3[01])\.[0-9]{1,3}\.[0-9]{1,3}\b"),
      form(
        "a private network address",
        r"\b100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.[0-9]{1,3}\.[0-9]{1,3}\b",
      ),
      form("a private network address", r"(?i)\b(?:f[cd][0-9a-f]{2}|fe80):"),
    ]
  });

  FORMS.as_slice()
}

/// The kinds a line holds, each found by its form.
fn personal_data_in(line: &str) -> BTreeSet<&'static str> {
  personal_data()
    .iter()
    .filter(|form| form.pattern.is_match(line))
    .map(|form| form.kind)
    .collect()
}

#[test]
fn depends_on_no_bmo_package_but_the_base_libraries() {
  let metadata = metadata();
  let packages = metadata["packages"].as_array().expect("metadata lists packages");
  let own: Vec<&str> = packages
    .iter()
    .map(|package| package["name"].as_str().expect("a package has a name"))
    .collect();
  let mut refused = Vec::new();
  for package in packages {
    let owner = package["name"].as_str().expect("a package has a name");
    for dependency in package["dependencies"].as_array().expect("a package lists its dependencies") {
      let name = dependency["name"].as_str().expect("a dependency has a name");
      let foreign_bmo = name.starts_with("bmo-") && !BASE_LIBRARIES.contains(&name) && !own.contains(&name);
      if foreign_bmo || OLD_SHARED_CRATES.contains(&name) {
        refused.push(format!("{owner} -> {name}"));
      }
    }
  }
  assert!(refused.is_empty(), "dependencies outside the base libraries: {refused:?}");
}

#[test]
fn no_package_takes_an_old_shared_crate_name() {
  let metadata = metadata();
  let taken: Vec<&str> = metadata["packages"]
    .as_array()
    .expect("metadata lists packages")
    .iter()
    .map(|package| package["name"].as_str().expect("a package has a name"))
    .filter(|name| OLD_SHARED_CRATES.contains(name))
    .collect();
  assert!(taken.is_empty(), "packages named after old shared crates: {taken:?}");
}

fn declaration() -> toml::Table {
  let text = text_of("testbed.toml").expect("testbed.toml is at the root");
  text.parse().expect("testbed.toml is TOML")
}

fn table<'a>(within: &'a toml::Table, key: &str) -> &'a toml::Table {
  within
    .get(key)
    .and_then(toml::Value::as_table)
    .unwrap_or_else(|| panic!("testbed.toml has `[{key}]`"))
}

fn text<'a>(within: &'a toml::Table, key: &str) -> &'a str {
  within
    .get(key)
    .and_then(toml::Value::as_str)
    .unwrap_or_else(|| panic!("`{key}` is a string"))
}

#[test]
fn the_testbed_declaration_is_in_the_testbeds_format_and_carries_no_version() {
  let declaration = declaration();
  let tables = ["test", "projects", "tools", "check", "build", "deploy", "runner", "events"];
  let strange: Vec<&String> = declaration
    .keys()
    .filter(|key| !["testbed", "name"].contains(&key.as_str()) && !tables.contains(&key.as_str()))
    .collect();

  assert_eq!(declaration.get("testbed").and_then(toml::Value::as_integer), Some(0));
  assert!(strange.is_empty(), "testbed.toml holds keys its format does not name: {strange:?}");
  for key in tables.iter().filter(|key| declaration.contains_key(**key)) {
    table(&declaration, key);
  }
  assert_eq!(
    table(&declaration, "test").get("languages").and_then(toml::Value::as_array),
    Some(&vec![toml::Value::from("cargo"), toml::Value::from("bun")]),
    "the unit checks its Cargo workspace and its console bundle"
  );
}

// One key of a unit file's section, as systemd reads it.
fn unit_key<'a>(unit: &'a ini::Ini, path: &str, section: &str, key: &str) -> &'a str {
  unit
    .section(Some(section))
    .and_then(|keys| keys.get(key))
    .unwrap_or_else(|| panic!("{path} has `{key}` in [{section}]"))
}

#[test]
fn the_service_unit_runs_the_declared_binary_over_the_linked_settings_as_the_bot() {
  let declaration = declaration();
  let deploy = table(&declaration, "deploy");
  let built: Vec<&str> = table(&declaration, "build")
    .get("binaries")
    .and_then(toml::Value::as_array)
    .expect("testbed.toml names the release's binaries")
    .iter()
    .filter_map(toml::Value::as_str)
    .collect();
  let link = deploy
    .get("env_links")
    .and_then(toml::Value::as_array)
    .expect("testbed.toml links the settings into each release")
    .iter()
    .filter_map(toml::Value::as_table)
    .find(|link| text(link, "link") == "settings.local.toml")
    .expect("the settings are linked into each release as settings.local.toml");
  let tree = format!("{}/{}", text(deploy, "root"), text(deploy, "link"));
  let in_unit = |path: &str| path.replace("~/", "%h/");
  let units: Vec<String> = tracked_files()
    .into_iter()
    .filter(|path| {
      let parts: Vec<&str> = path.split('/').collect();
      parts.len() == 2 && parts[0] == "deploy" && parts[1].ends_with(".service")
    })
    .collect();
  let mut bots = Vec::new();

  assert_eq!(
    deploy.get("units").and_then(toml::Value::as_array),
    Some(&vec![toml::Value::from("deploy/*.service")])
  );
  assert!(built.contains(&"bmo-voc"), "the release holds the service's binary");
  assert_eq!(units, ["deploy/bmo-voc.service"]);

  for path in &units {
    let unit = ini::Ini::load_from_file(Path::new(ROOT).join(path)).expect("a unit file reads");
    assert_eq!(unit_key(&unit, path, "Service", "WorkingDirectory"), in_unit(&tree));
    assert_eq!(
      unit_key(&unit, path, "Service", "ExecStart"),
      in_unit(&format!("{tree}/bin/bmo-voc serve"))
    );
    assert_eq!(
      unit_key(&unit, path, "Unit", "ConditionPathExists"),
      in_unit(&format!("{}/{}", text(deploy, "data_root"), text(link, "target")))
    );
    if unit_key(&unit, path, "Install", "X-BMO-Deploy") == "bot" {
      bots.push(path.clone());
    }
  }

  assert_eq!(bots, ["deploy/bmo-voc.service"], "the service is the repository's one bot unit");
  assert_eq!(text(deploy, "health"), format!("{tree}/bin/bmo-voc health"));
}

#[test]
fn only_cargo_manifests_at_the_root() {
  let found: Vec<String> = fs::read_dir(ROOT)
    .expect("the repository root is readable")
    .map(|entry| entry.expect("a root entry is readable").file_name().to_string_lossy().into_owned())
    .filter(|name| OTHER_MANIFESTS.contains(&name.as_str()))
    .collect();
  assert!(found.is_empty(), "the root holds {found:?}; only Cargo manifests belong there");
}

#[test]
fn no_directory_is_named_after_its_parent() {
  let twins: Vec<String> = tracked_files()
    .into_iter()
    .filter(|path| {
      let parts: Vec<Component> = Path::new(path).components().collect();
      parts.windows(2).take(parts.len().saturating_sub(2)).any(|pair| pair[0] == pair[1])
    })
    .collect();
  assert!(twins.is_empty(), "these paths repeat a directory inside itself: {twins:?}");
}

/// The first cell of every body row of the README's table whose first header cell is `Folder`, without the
/// code span's marks and the trailing slash.
fn readme_folders(readme: &str) -> BTreeSet<String> {
  let mut folders = BTreeSet::new();
  let mut in_head = false;
  let mut cell = 0usize;
  let mut text = String::new();
  let mut is_folder_table = false;
  let mut header_first = None::<String>;
  for event in Parser::new_ext(readme, Options::ENABLE_TABLES) {
    match event {
      Event::Start(Tag::Table(_)) => {
        is_folder_table = false;
        header_first = None;
      }
      Event::Start(Tag::TableHead) => {
        in_head = true;
        cell = 0;
      }
      Event::End(TagEnd::TableHead) => {
        in_head = false;
        is_folder_table = header_first.as_deref() == Some("Folder");
      }
      Event::Start(Tag::TableRow) => cell = 0,
      Event::Start(Tag::TableCell) => text.clear(),
      Event::End(TagEnd::TableCell) => {
        if in_head && cell == 0 {
          header_first = Some(text.trim().to_owned());
        } else if !in_head && is_folder_table && cell == 0 {
          folders.insert(text.trim().trim_end_matches('/').to_owned());
        }
        cell += 1;
      }
      Event::Text(piece) | Event::Code(piece) => text.push_str(&piece),
      _ => {}
    }
  }
  folders
}

/// Whether a folder is a tool's hidden one, such as the code graph's settings, which the testbed's `layout` quick
/// check holds the README to and a copy of the tree without them leaves out.
fn hidden(folder: &str) -> bool {
  folder.starts_with('.')
}

#[test]
fn the_readme_names_every_top_level_folder_and_no_other() {
  let standing: BTreeSet<String> = tracked_files()
    .into_iter()
    .filter_map(|path| path.split_once('/').map(|(folder, _)| folder.to_owned()))
    .filter(|folder| !hidden(folder))
    .collect();
  let named: BTreeSet<String> = readme_folders(&text_of("README.md").expect("README.md is at the root"))
    .into_iter()
    .filter(|folder| !hidden(folder))
    .collect();
  assert_eq!(named, standing, "the README's folder table and the top-level folders differ");
}

#[test]
fn no_hangul_in_rust_source() {
  let found: Vec<String> = tracked_texts()
    .into_iter()
    .filter(|(path, _)| path.ends_with(".rs") && ["src/", "tests/"].iter().any(|top| path.starts_with(top)))
    .filter(|(_, text)| text.chars().any(is_hangul))
    .map(|(path, _)| path)
    .collect();
  assert!(found.is_empty(), "Rust files holding Hangul: {found:?}");
}

#[test]
fn no_personal_data_in_tracked_files() {
  let mut found = Vec::new();
  for (path, text) in tracked_texts() {
    for (index, line) in text.lines().enumerate() {
      for kind in personal_data_in(line) {
        found.push(format!("{path}:{}: {kind}", index + 1));
      }
    }
  }
  assert!(found.is_empty(), "personal data in tracked files (PERSONAL-DATA.md): {found:#?}");
}

#[test]
fn the_personal_data_check_finds_each_form() {
  let (at, user, folder) = ("@", "tester", "notes");
  let (prefix, middle, last) = ("010", "1234", "5678");
  let (network, unit) = ("10", "0");
  let body = "a".repeat(36);

  let examples = [
    ("a mail address", format!("{user}{at}tester.test")),
    ("a home path", format!("/home/{user}/{folder}")),
    ("a telephone number", format!("{prefix}-{middle}-{last}")),
    ("a token", format!("ghp_{body}")),
    ("a private network address", format!("{network}.{unit}.{unit}.1")),
  ];

  for (kind, text) in examples {
    assert!(personal_data_in(&text).contains(kind), "{kind} is not found in its own form");
  }

  for text in ["a neutral line", "writer-a", "board:a", "release 1.2.3"] {
    assert!(personal_data_in(text).is_empty(), "the check flags the neutral line {text}");
  }
}
