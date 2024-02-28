use assert_cmd::Command;

#[cfg(feature = "msgpack")]
fn send_data_rpc(cmd: &mut Command) {
    let rpc = (2, "send_data", vec!["Hello, world!"]);

    cmd.write_stdin(rmp_serde::to_vec(&rpc).unwrap());
}

#[cfg(feature = "json-rpc")]
fn send_data_rpc(cmd: &mut Command) {
    use serde_json::json;

    let rpc = vec![
        json!(1),
        json!({ "method": "send_data", "params": vec![json!("Hello, World!")]}),
    ];

    cmd.write_stdin(serde_json::to_vec(&rpc).unwrap());
}

#[test]
fn rpc() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("--no-auto-open");

    send_data_rpc(&mut cmd);

    cmd.assert().success();
}
