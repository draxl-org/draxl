#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(flag) = args.next() else {
        return Err(usage());
    };
    if flag != "--root" {
        return Err(usage());
    }
    let Some(root) = args.next() else {
        return Err(usage());
    };

    draxl_itest::mcp::serve_stdio(root)
        .await
        .map_err(|err| format!("draxl-itest-mcp failed: {err}"))
}

fn usage() -> String {
    "usage:
  draxl-itest-mcp --root <workspace>"
        .to_owned()
}
