use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;

use crate::net::ToSocketAddrs;
use crate::net::socket::{Domain, Protocol, Socket, Type};

pub struct UdpSocket(Socket);

impl UdpSocket {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let address = addr.to_socket_addrs().await?.next().ok_or(io::Error::new(
            ErrorKind::InvalidInput,
            "could not resolve to any address",
        ))?;

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

    pub async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: &A) -> io::Result<usize> {
        let address = addr.to_socket_addrs().await?.next().ok_or(io::Error::new(
            ErrorKind::InvalidInput,
            "could not resolve to any address",
        ))?;

        self.0.send_to(buf, &address.into()).await
    }
}
