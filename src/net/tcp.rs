use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;

use crate::net::ToSocketAddrs;
use crate::net::socket::{Domain, Protocol, Socket, Type};

pub struct TcpStream(Socket);

impl TcpStream {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let address = addr.to_socket_addrs().await?.next().ok_or(io::Error::new(
            ErrorKind::InvalidInput,
            "could not resolve to any address",
        ))?;

        let socket = Socket::new(
            Domain::for_address(address),
            Type::STREAM,
            Some(Protocol::TCP),
        )?;

        socket.connect(&address.into()).await?;

        Ok(Self(socket))
    }

    pub async fn read(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<usize> {
        self.0.recv(buf).await
    }

    pub async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf).await
    }
}
