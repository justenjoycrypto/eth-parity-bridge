// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.
extern crate solc;

use std::process::Command;

fn main() {
	// rerun build script if bridge contract has changed.
	// without this cargo doesn't since the bridge contract
	// is outside the crate directories
	println!("cargo:rerun-if-changed=../arbitrary/contracts/bridge.sol");

	// make last git commit hash (`git rev-parse HEAD`)
	// available via `env!("GIT_HASH")` in sources
	let output = Command::new("git")
		.args(&["rev-parse", "HEAD"])
		.output()
		.expect("`git rev-parse HEAD` failed to run. run it yourself to verify. file an issue if this persists");
	let git_hash = String::from_utf8(output.stdout).unwrap();
	println!("cargo:rustc-env=GIT_HASH={}", git_hash);

	// make solc version used to compile contracts (`solc --version`)
	// available via `env!("SOLC_VERSION")` in sources
	let output = Command::new("solc").args(&["--version"]).output().expect(
		"`solc --version` failed to run. run it yourself to verify. file an issue if this persists",
	);
	let output_string = String::from_utf8(output.stdout).unwrap();
	let solc_version = output_string.lines().last().unwrap();
	println!("cargo:rustc-env=SOLC_VERSION={}", solc_version);

	// compile contracts for inclusion with ethabis `use_contract!`
	solc::solc_compile("../arbitrary/contracts/bridge.sol", "../compiled_contracts").unwrap();
}
