extern crate pcs_protocol;
use pcs_protocol::Message;

extern crate clap;
use clap::{ App, Arg };

extern crate futures;
use futures::{ future::Future, stream::Stream };

extern crate hyper;
use hyper::{ Body, Response, Server, service };

#[macro_use] extern crate log;
extern crate pretty_env_logger;

extern crate rustls;
extern crate tokio_core;
use tokio_core::{ net::TcpListener, reactor::Core };

extern crate tokio_io;
use tokio_io::io;

extern crate tokio_rustls;
use tokio_rustls::ServerConfigExt;

extern crate webpki;
extern crate webpki_roots;

use std::net::ToSocketAddrs;

mod ssl;

fn main() -> Result<(), Box<std::error::Error>> {
    pretty_env_logger::init();

    let m = App::new("PCS server")
        .author("Tom Almeida, tommoa256@gmail.com")
        .version("0.1")
        .about("PCS server")
        .arg(Arg::with_name("http_host")
             .short("h")
             .long("host")
             .default_value("localhost")
        )
        .arg(Arg::with_name("judge_host")
             .short("j")
             .long("judge")
             .default_value("localhost")
        )
        .arg(Arg::with_name("http_port")
             .short("p")
             .long("port")
             .default_value("8080")
        )
        .arg(Arg::with_name("judge_port")
             .short("u")
             .long("judge-port")
             .default_value("11286")
        )
        .arg(Arg::with_name("cert")
             .short("c")
             .long("certificate")
             .takes_value(true)
             .required(true)
        )
        .arg(Arg::with_name("key")
             .short("k")
             .long("private-key")
             .takes_value(true)
             .required(true)
        )
        .get_matches();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let judge_host = m.value_of("judge_host").unwrap();
    let judge_port = m.value_of("judge_port").unwrap().parse().unwrap();
    let judge_addr =
        (judge_host, judge_port)
        .to_socket_addrs().unwrap().next().unwrap();
    let judge_listener = TcpListener::bind(&judge_addr, &handle).unwrap();

    let http_host = m.value_of("http_host").unwrap();
    let http_port = m.value_of("http_port").unwrap().parse().unwrap();
    let http_addr =
        (http_host, http_port)
        .to_socket_addrs().unwrap().next().unwrap();
    let http_listener = TcpListener::bind(&http_addr, &handle).unwrap();

    let cert = m.value_of("cert").unwrap();
    let pkey = m.value_of("key").unwrap();

    let arc_config = ssl::setup(cert, pkey);

    let config = arc_config.clone();

    let http_server = Server::builder(
        // This is more complicated than it needs to be, but \o/
        http_listener.incoming()
            .map(|(sock, addr)| {
                info!("Connected from {}", addr);
                config.accept_async(sock).into_stream()
            }
            ).flatten())
        .serve(|| service::service_fn_ok(|_req| {
            info!("Connected");
            Response::new(Body::from("Hello world!"))
        })).map_err(|e| Box::new(e) as Box<std::error::Error>);

    let config = arc_config.clone();
    let handle = core.handle();
    let judge_server = judge_listener.incoming().for_each(move |(sock, addr)| {
        info!("Connection to judge server from {}", addr);
        let handle_conn = config.accept_async(sock)
            .and_then(|mut stream| {
                let mut stream = pcs_protocol::CodedOutputStream::new(&mut stream);
                futures::future::result(
                    pcs_protocol::Verify::new()
                    .write_to_with_cached_sizes(&mut stream)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                )
            })
            .map(move |_| info!("Accept: {}", addr))
            .map_err(move |err| error!("Couldn't get client SSL {}! {}", addr, err));
        handle.spawn(handle_conn);
        Ok(())
    }).map_err(|e| Box::new(e) as Box<std::error::Error>);

    info!("Starting judge server at {}!", judge_addr);
    info!("Starting http server at {}!", http_addr);
    core.run(http_server.select(judge_server))
        .map(|_| info!("Exiting"))
        .map_err(|(e, _)| e)
}
