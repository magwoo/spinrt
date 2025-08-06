use std::future::Future;
use std::net::SocketAddr;
use std::task::Poll;
use std::{io, option, vec};

pub use self::tcp::TcpStream;
pub use self::udp::UdpSocket;

pub mod socket;
pub mod tcp;
pub mod udp;

pub trait ToSocketAddrs {
    type Iter: Iterator<Item = SocketAddr>;

    fn to_socket_addrs(&self) -> impl Future<Output = io::Result<Self::Iter>> + Send;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = option::IntoIter<SocketAddr>;

    async fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        Ok(Some(*self).into_iter())
    }
}

impl ToSocketAddrs for &str {
    type Iter = vec::IntoIter<SocketAddr>;

    async fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        let host = self.to_string();

        let handle = crate::spawn_blocking(move || std::net::ToSocketAddrs::to_socket_addrs(&host));

        core::future::poll_fn(|_| match handle.poll_nonblocking() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        })
        .await
    }
}
