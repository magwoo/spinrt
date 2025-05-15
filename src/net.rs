use std::io::{self};
use std::net::{ToSocketAddrs, UdpSocket as StdUdpSocket};
use std::task::Poll;

pub struct UdpSocket(StdUdpSocket);

impl UdpSocket {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let std_socket = StdUdpSocket::bind(addr)?;
        std_socket.set_nonblocking(true)?;

        Ok(Self(std_socket))
    }

    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        core::future::poll_fn(|_| match self.0.recv(buf) {
            Ok(readed) => Poll::Ready(Ok(readed)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
        .await
    }
}
