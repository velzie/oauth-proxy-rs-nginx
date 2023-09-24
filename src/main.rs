use hmac::{Hmac, Mac};

use getopts::Options;
use std::env;
use std::fs;
use std::net::SocketAddr;

mod oauth;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("", "authorized-users", "comma separated list of github user IDs (find uid at https://api.github.com/users/your_username)", "");
    opts.optopt("", "authorized-orgs", "comma separated list of github organization IDs (find uid at https://api.github.com/orgs/your_organization)", "");
    opts.optopt("", "client-secret", "oauth client secret", "");
    opts.optopt("", "client-id", "oauth client ID", "");
    opts.optopt("k", "key", "set path to JWT secret", "");
    opts.optopt("p", "port", "port to bind to", "8080");
    opts.optopt("", "host", "host to bind to", "0.0.0.0");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };
    if matches.opt_present("h")
        || !matches.opt_present("k")
        || !matches.opt_present("p")
        || !matches.opt_present("host")
        || !matches.opt_present("client-secret")
        || !matches.opt_present("client-id")
    {
        print_usage(&program, opts);
        return;
    }
    let keyfile = matches.opt_str("k").unwrap();
    let port = matches.opt_str("p").unwrap();
    let host = matches.opt_str("host").unwrap();

    let client_secret = matches.opt_str("client-secret").unwrap();
    let client_id = matches.opt_str("client-id").unwrap();

    let authorized_users = matches
        .opt_str("authorized-users")
        .unwrap_or(String::new())
        .split(",")
        .map(|f| f.to_string())
        .collect::<Vec<String>>();
    let authorized_orgs = matches
        .opt_str("authorized-orgs")
        .unwrap_or(String::new())
        .split(",")
        .map(|f| f.to_string())
        .collect::<Vec<String>>();

    let rawkey = fs::read_to_string(keyfile).unwrap();

    let key = Hmac::new_from_slice(rawkey.as_bytes()).unwrap();

    let authapp = oauth::app(
        client_id,
        client_secret,
        key,
        authorized_users,
        authorized_orgs,
    );

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();
    println!(
        "auth portal listening on {}",
        listener.local_addr().unwrap()
    );

    axum::serve(
        listener,
        authapp.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
