extern crate assert_cli;
extern crate hammond;

// Notes:
// The following tests will use your systems local hammond db.
// There are no test for failure, cause the behavior have not
// been defined/though yet.

#[test]
fn test_update() {
    assert_cli::Assert::main_binary()
        .with_args(&["--update"])
        .unwrap();
}

#[test]
fn test_download() {
    assert_cli::Assert::main_binary()
        .with_args(&["--download"])
        .unwrap();
}

#[test]
fn test_add() {
    assert_cli::Assert::main_binary()
        .with_args(&[
            "--add",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
        ])
        .unwrap();
}
