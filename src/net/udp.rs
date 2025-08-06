use std::io::{self};
use std::mem::MaybeUninit;

use crate::net::ToSocketAddrs;
use crate::net::socket::{Domain, Protocol, Socket, Type};

pub struct UdpSocket(Socket);

impl UdpSocket {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let address = addr
            .to_socket_addrs()
            .await?
            .next()
            .ok_or(io::Error::other("missing addr"))?;

        let socket = Socket::new(
            Domain::for_address(address),
            Type::DGRAM,
            Some(Protocol::UDP),
        )?;

        socket.bind(&address.into()).await?;

        Ok(Self(socket))
    }

    pub async fn recv(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<usize> {
        self.0.recv(buf).await
    }

    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf).await
    }

    // pub async fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> io::Result<usize> {
    //     self.0.send_to(buf, addr.into()).await
    // }
}
