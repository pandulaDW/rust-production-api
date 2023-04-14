use std::net;
use zero2prod::startup::run;

fn main() -> std::io::Result<()> {
    let listener = net::TcpListener::bind("127.0.0.1:8000")?;
    let server = run(listener)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(server)
}
