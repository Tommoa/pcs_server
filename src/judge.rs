extern crate pcs_protocol;
use pcs_protocol::{ MsgType, SerDe };

extern crate futures;
use futures::prelude::*;

extern crate libc;

use std::io::{ Error, ErrorKind, Read, Write };
use std::sync::{ Arc, Mutex };

pub struct Judge<S> {
    pub judge: Arc<Mutex<S>>,
    pub in_fd: i32
}
impl<S: Read + Write> Stream for Judge<S> {
    type Item = (MsgType, Arc<Mutex<S>>);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        use std::mem;
        let mut pfd: libc::pollfd = unsafe { mem::zeroed() };
        pfd.fd = self.in_fd;
        pfd.events |= libc::POLLIN;
        unsafe { libc::poll(&mut pfd, 1, 0) };
        if pfd.revents & libc::POLLIN > 0 {
            let msg = MsgType::deserialize(&mut *self.judge.lock().unwrap())?;
            Ok(Async::Ready(Some((msg, self.judge.clone()))))
        } else if pfd.revents & libc::POLLHUP > 0 {
            Ok(Async::Ready(None))
        } else if pfd.revents & libc::POLLERR > 0 {
            Err(Error::new(ErrorKind::BrokenPipe, "POLLERR"))
        } else if pfd.revents & libc::POLLNVAL > 0 {
            Err(Error::new(ErrorKind::InvalidData, "POLLNVAL"))
        } else {
            Ok(Async::NotReady)
        }
    }
}
