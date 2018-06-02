extern crate rustls;
use rustls::internal::pemfile::{ certs, pkcs8_private_keys, rsa_private_keys };
use rustls::{ Certificate, PrivateKey };

extern crate webpki;

extern crate webpki_roots;

use std::sync::Arc;
use std::fs;
use std::io::BufReader;

fn load_certs(path: &str) -> Vec<Certificate> {
    certs(
        &mut BufReader::new(
            fs::File::open(path)
                .map_err(|e| error!("Error getting certificate file {}: {}", path, e)).unwrap())).unwrap()
}

fn load_keys(path: &str) -> Vec<PrivateKey> {
    pkcs8_private_keys(
        &mut BufReader::new(
            fs::File::open(path)
                .map_err(|e| error!("Error getting private key file {}: {}", path, e)).unwrap()))
    .or_else(|()|
        rsa_private_keys(
            &mut BufReader::new(
                fs::File::open(path).unwrap()))).unwrap()
}

pub fn setup(cert: &str, key: &str) -> Arc<rustls::ServerConfig> {
    let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());

    config.set_single_cert(load_certs(cert), load_keys(key).remove(0));
    Arc::new(config)
}
