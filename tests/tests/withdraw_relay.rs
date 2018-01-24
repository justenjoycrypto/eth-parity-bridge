extern crate futures;
extern crate bridge;
#[macro_use]
extern crate tests;

use bridge::bridge::create_withdraw_relay;

// 1 signature required. relay polled twice.
// no CollectedSignatures on ForeignBridge.
// no relay.
test_app_stream! {
	name => withdraw_relay_no_log_no_relay,
	database => Database::default(),
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_withdraw_relay(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[]"#;
	]
}

// 2 signatures required. relay polled twice.
// single CollectedSignatures log present. message value covers relay cost.
// authority not responsible.
// message is ignored.
test_app_stream! {
	name => withdraw_relay_single_log_authority_not_responsible_no_relay,
	database => Database::default(),
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_withdraw_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[{"address":"0x0000000000000000000000000000000000000000","topics":["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
	]
}

// 2 signatures required. relay polled twice.
// single CollectedSignatures log present. message value covers relay cost.
// message gets relayed.
test_app_stream! {
	name => withdraw_relay_single_log_sufficient_value_relay,
	database => Database::default(),
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0xaff3454fce5edbc8cca8697c15331677e6ebcccc",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 2;
	txs => Transactions::default(),
	init => |app, db| create_withdraw_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		// call to `isValueInMessageLargeEnoughToCoverWithdrawCost`
		"eth_call" =>
			req => r#"[{"data":"0x6498d59000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			// respond with `true`
			res => r#""0x0000000000000000000000000000000000000000000000000000000000000001""#;
		// `withdraw`
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x9ce318f6000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000001a00000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000001100000000000000000000000000000000000000000000000000000000000000220000000000000000000000000000000000000000000000000000000000000002111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000002111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0x0","gasPrice":"0x0","to":"0x0000000000000000000000000000000000000000"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
	],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[{"address":"0x0000000000000000000000000000000000000000","topics":["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
		// call to `message`
		"eth_call" =>
			req => r#"[{"data":"0x490a32c600000000000000000000000000000000000000000000000000000000000000f0","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000""#;
		// calls to `signature`
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000000""#;
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222200000000000000000000000000000000000000000000000000000000000000""#;
	]
}

// 2 signatures required. relay polled twice.
// single CollectedSignatures log present. message value doesn't cover cost.
// message is ignored.
test_app_stream! {
	name => withdraw_relay_single_log_insufficient_value_no_relay,
	database => Database::default(),
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0xaff3454fce5edbc8cca8697c15331677e6ebcccc",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 2;
	txs => Transactions::default(),
	init => |app, db| create_withdraw_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		// call to `isValueInMessageLargeEnoughToCoverWithdrawCost`
		"eth_call" =>
			req => r#"[{"data":"0x6498d59000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			// respond with `false`
			res => r#""0x0000000000000000000000000000000000000000000000000000000000000000""#;
		// no `withdraw`
	],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[{"address":"0x0000000000000000000000000000000000000000","topics":["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
		// call to `message`
		"eth_call" =>
			req => r#"[{"data":"0x490a32c600000000000000000000000000000000000000000000000000000000000000f0","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000""#;
		// calls to `signature`
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000000""#;
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222200000000000000000000000000000000000000000000000000000000000000""#;
	]
}

// like `withdraw_relay_single_log_sufficient_value_relay`
// but with explicit gas
test_app_stream! {
	name => withdraw_relay_explicit_gas,
	database => Database::default(),
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0xaff3454fce5edbc8cca8697c15331677e6ebcccc",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 2;
	txs => Transactions {
		withdraw_relay: TransactionConfig {
			gas: 0x10,
			gas_price: 0x20,
		},
		..Default::default()
	},
	init => |app, db| create_withdraw_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		// call to `isValueInMessageLargeEnoughToCoverWithdrawCost`
		"eth_call" =>
			req => r#"[{"data":"0x6498d59000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			// true
			res => r#""0x0000000000000000000000000000000000000000000000000000000000000001""#;
		// `withdraw`
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x9ce318f6000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000001a00000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000001100000000000000000000000000000000000000000000000000000000000000220000000000000000000000000000000000000000000000000000000000000002111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000002111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0x10","gasPrice":"0x20","to":"0x0000000000000000000000000000000000000000"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
	],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[{"address":"0x0000000000000000000000000000000000000000","topics":["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
		// call to `message`
		"eth_call" =>
			req => r#"[{"data":"0x490a32c600000000000000000000000000000000000000000000000000000000000000f0","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000""#;
		// calls to `signature`
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000000","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000000""#;
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000001","to":"0x0000000000000000000000000000000000000000"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222200000000000000000000000000000000000000000000000000000000000000""#;
	]
}

// like `withdraw_relay_single_log_sufficient_value_relay`
// but with explicit contract addresses
test_app_stream! {
	name => withdraw_relay_single_explicit_contract_addresses,
	database => Database {
		home_contract_address: "0x00000000000000000000000000000000000000dd".parse().unwrap(),
		foreign_contract_address: "0x00000000000000000000000000000000000000ee".parse().unwrap(),
		..Default::default()
	},
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0xaff3454fce5edbc8cca8697c15331677e6ebcccc",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 2;
	txs => Transactions::default(),
	init => |app, db| create_withdraw_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		// call to `isValueInMessageLargeEnoughToCoverWithdrawCost`
		"eth_call" =>
			req => r#"[{"data":"0x6498d59000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","to":"0x00000000000000000000000000000000000000dd"},"latest"]"#,
			// true
			res => r#""0x0000000000000000000000000000000000000000000000000000000000000001""#;
		// `withdraw`
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x9ce318f6000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000001a00000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000001100000000000000000000000000000000000000000000000000000000000000220000000000000000000000000000000000000000000000000000000000000002111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000002111111111111111111111111111111111111111111111111111111111111111122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0x0","gasPrice":"0x0","to":"0x00000000000000000000000000000000000000dd"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
	],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x00000000000000000000000000000000000000ee"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],null,null,null]}]"#,
			res => r#"[{"address":"0x00000000000000000000000000000000000000ee","topics":["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
		// call to `message`
		"eth_call" =>
			req => r#"[{"data":"0x490a32c600000000000000000000000000000000000000000000000000000000000000f0","to":"0x00000000000000000000000000000000000000ee"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000054333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333000000000000000000000000""#;
		// calls to `signature`
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000000","to":"0x00000000000000000000000000000000000000ee"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000000""#;
		"eth_call" =>
			req => r#"[{"data":"0x1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000001","to":"0x00000000000000000000000000000000000000ee"},"latest"]"#,
			res => r#""0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000041222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222222200000000000000000000000000000000000000000000000000000000000000""#;
	]
}
