extern crate pcs_protocol;
use pcs_protocol::{ MsgType, SerDe };

extern crate clap;
use clap::{ App, Arg };

extern crate futures;
use futures::{ future::Future, stream::Stream };

extern crate hyper;
use hyper::{ Body, Response, Server, service };

extern crate libc;

#[macro_use] extern crate log;
extern crate pretty_env_logger;

extern crate rustls;
extern crate tokio_core;
use tokio_core::{ net::TcpListener, reactor::Core };

extern crate tokio_rustls;
use tokio_rustls::ServerConfigExt;

extern crate webpki;
extern crate webpki_roots;

use std::net::ToSocketAddrs;
use std::sync::{ Arc, Mutex };

mod ssl;
mod judge;

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
        .arg(Arg::with_name("https_port")
             .short("s")
             .long("ssl_port")
             .default_value("443")
        )
        /*
         * .arg(Arg::with_name("http_port")
         *      .short("p")
         *      .long("port")
         *      .default_value("80")
         * )
         */
        .arg(Arg::with_name("judge_host")
             .short("j")
             .long("judge")
             .default_value("localhost")
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

    /*
     * let http_port = m.value_of("http_port").unwrap().parse().unwrap();
     * let http_addr =
     *     (http_host, http_port)
     *     .to_socket_addrs().unwrap().next().unwrap();
     * let http_listener = TcpListener::bind(&http_addr, &handle).unwrap();
     */

    let https_port = m.value_of("https_port").unwrap().parse().unwrap();
    let https_addr =
        (http_host, https_port)
        .to_socket_addrs().unwrap().next().unwrap();
    let https_listener = TcpListener::bind(&https_addr, &handle).unwrap();

    let cert = m.value_of("cert").unwrap();
    let pkey = m.value_of("key").unwrap();

    let arc_config = ssl::setup(cert, pkey);

    let config = arc_config.clone();

    // TODO: Make this server somehow join with an http option
    let http_server = Server::builder(
        // This is more complicated than it needs to be, but \o/
        https_listener.incoming()
            .map(|(sock, addr)| {
                info!("Connected from {}", addr);
                config.accept_async(sock).into_stream()
            })
            .flatten())
        .serve(|| service::service_fn_ok(|_req| {
            // TODO: Actually process the request
            info!("Connected");
            Response::new(Body::from("Hello world!"))
        })).map_err(|e| Box::new(e) as Box<std::error::Error>);

    let config = arc_config.clone();
    let handle = core.handle();
    let judge_server = judge_listener.incoming().for_each(move |(sock, addr)| {
        info!("Connection to judge server from {}", addr);
        let handle_conn = config.accept_async(sock)
            .map(move |stream| {
                info!("Accept: {}", addr);
                stream
            })
            .and_then(|mut stream| {
                MsgType::Verify.serialize(&mut stream).unwrap();
                let fd = {
                    use std::os::unix::io::AsRawFd;
                    stream.get_ref().0.as_raw_fd()
                };
                judge::Judge {
                    judge: Arc::new(Mutex::new(stream)),
                    in_fd: fd
                }.for_each(|(_msg, _stream)| {
                    // TODO: Make this process the msg
                    Ok(())
                })
            }).map_err(move |err| {
                error!("Couldn't get client SSL {}! {}", addr, err);
                ()
            });
        handle.spawn(handle_conn);
        Ok(())
    }).map_err(|e| Box::new(e) as Box<std::error::Error>);

    info!("Starting judge server at {}!", judge_addr);
    info!("Starting https server at {}!", https_addr);
    core.run(http_server.select(judge_server))
        .map(|_| info!("Exiting"))
        .map_err(|(e, _)| e)
}
