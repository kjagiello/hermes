use argo_hermes::k8s::templates::K8sTemplateRegistry;
use argo_hermes::server::filters;
use argo_hermes::services::registries::DefaultServiceRegistry;
use clap::{App, Arg};
use std::io;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::process;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

fn main() -> io::Result<()> {
    let matches = App::new(PKG_NAME)
        .bin_name(PKG_NAME)
        .version(PKG_VERSION)
        .author(PKG_AUTHORS)
        .about("Notifications for your Argo Workflows.")
        .arg(
            Arg::new("host")
                .short('h')
                .takes_value(true)
                .help("Host to bind Hermes to [default: 0.0.0.0]"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .takes_value(true)
                .help("Port to bind Hermes to [default: 3030]"),
        )
        .get_matches();
    let port: u16 = matches
        .value_of("port")
        .unwrap_or("3030")
        .parse()
        .unwrap_or_else(|_| {
            println!("Specified port is not in the valid range (1-65535)");
            process::exit(1);
        });
    let addr: SocketAddr = {
        let default_host = format!("0.0.0.0:{}", port);
        matches
            .value_of("host")
            .map(|host| format!("{}:{}", host, port))
            .unwrap_or(default_host)
            .to_socket_addrs()
            .unwrap_or_else(|err| {
                println!("Specified host is not valid: {}", err);
                process::exit(1);
            })
            .next()
            .unwrap_or_else(|| {
                println!("The given host was not resolvable");
                process::exit(1);
            })
    };
    serve(addr);
    Ok(())
}

#[tokio::main]
async fn serve(addr: SocketAddr) {
    let service_registry = DefaultServiceRegistry::with_default_services();
    let template_registry = K8sTemplateRegistry::new()
        .await
        .expect("Failed to init k8s template registry");

    let api = filters::routes(service_registry, template_registry);
    let (addr, server) = warp::serve(api)
        .try_bind_with_graceful_shutdown(addr, async {
            tokio::signal::ctrl_c()
                .await
                .expect("http_server: Failed to listen for CRTL+c");
            println!("Shutting down the server");
        })
        .unwrap_or_else(|e| {
            println!("Failed to start the server: {}", e);
            std::process::exit(1);
        });

    println!("Starting the server at {:?}", addr);
    tokio::task::spawn(server).await.expect("hello ");
}
