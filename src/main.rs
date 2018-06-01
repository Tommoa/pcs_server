extern crate pcs_protocol;
use pcs_protocol::{ MsgType, SerDe };

extern crate clap;
#[macro_use] extern crate log;

extern crate futures;
use futures::{ future::Future, stream::Stream };

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

fn main() {
    pretty_env_logger::init();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    
    let judge_addr = "localhost:11286".to_socket_addrs().unwrap().next().unwrap();
    let judge_listener = TcpListener::bind(&judge_addr, &handle).unwrap();

    let http_addr = "localhost:8080".to_socket_addrs().unwrap().next().unwrap();
    let http_listener = TcpListener::bind(&http_addr, &handle).unwrap();

    let arc_config = ssl::setup("/home/tommoa/Dropbox/Projects/pcs_server/cert.pem", "/home/tommoa/Dropbox/Projects/pcs_server/key.pem");

    let config = arc_config.clone();
    let http_server = http_listener.incoming().for_each(move |(sock, addr)| {
        info!("Connection to http server from {}", addr);
        let handle_conn = config.accept_async(sock)
            .and_then(|stream| io::write_all(
                    stream,
                    &b"HTTP/1.0 200 ok\r\n\
                    Connection: close\r\n\
                    Content-length: 12\r\n\
                    \r\n\
                    Hello world!"[..]
                    ))
            .and_then(|(stream, _)| io::flush(stream))
            .map(move |_| debug!("Accept: {}", addr))
            .map_err(move |err| error!("{} - {}", err, addr));
        handle.spawn(handle_conn);
        Ok(())
    });
    let config = arc_config.clone();
    let handle = core.handle();
    let judge_server = judge_listener.incoming().for_each(move |(sock, addr)| {
        info!("Connection to judge server from {}", addr);
        let handle_conn = config.accept_async(sock)
            .and_then(|stream| {
                let mut v = Vec::new();
                MsgType::Accept.serialize(&mut v);
                io::write_all(stream, v)
            })
        .and_then(|(stream, _)| io::flush(stream))
            .map(move |_| info!("Accept: {:?}", addr))
            .map_err(move |err| error!("Couldn't get client SSL {}! {}", addr, err));
        handle.spawn(handle_conn);
        Ok(())
    });

    info!("Starting judge server at {}!", judge_addr);
    info!("Starting http server at {}!", http_addr);
    core.run(judge_server.select(http_server));
}
