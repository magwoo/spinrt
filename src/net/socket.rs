use socket2::Socket as Socket2;
use std::ffi::c_int;
use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;
use std::net::Shutdown;
use std::task::Poll;
use std::time::Duration;

pub use socket2::*;

pub struct Socket(Socket2);

impl Socket {
    pub fn new(domain: Domain, ty: Type, protocol: Option<Protocol>) -> io::Result<Self> {
        let sock2 = Socket2::new(domain, ty, protocol)?;
        sock2.set_nonblocking(true)?;

        Ok(Self(sock2))
    }

    pub async fn bind(&self, address: &SockAddr) -> io::Result<()> {
        nonblocking_future(|| self.0.bind(address)).await
    }

    pub async fn connect(&self, address: &SockAddr) -> io::Result<()> {
        match self.0.connect(address) {
            Ok(_) => return Ok(()),
            Err(err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(err) if err.raw_os_error().is_some_and(|c| c == 36) => (),
            Err(err) => return Err(err),
        }

        core::future::poll_fn(move |_| match self.0.send(&[]) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(err) if err.kind() == ErrorKind::NotConnected => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        })
        .await
    }

    pub async fn connect_timeout(&self, _addr: &SockAddr, _timeout: Duration) -> io::Result<()> {
        todo!()
    }

    pub async fn listen(&self, backlog: c_int) -> io::Result<()> {
        nonblocking_future(|| self.0.listen(backlog)).await
    }

    pub async fn accept(&self) -> io::Result<(Socket, SockAddr)> {
        let (sock2, addr) = nonblocking_future(|| self.0.accept()).await?;

        Ok((Self(sock2), addr))
    }

    pub fn local_addr(&self) -> io::Result<SockAddr> {
        self.0.local_addr()
    }

    pub fn peer_addr(&self) -> io::Result<SockAddr> {
        self.0.peer_addr()
    }

    pub fn r#type(&self) -> io::Result<Type> {
        self.0.r#type()
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }

    pub async fn recv(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<usize> {
        nonblocking_future(|| self.0.recv(buf)).await
    }

    pub async fn recv_from(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<(usize, SockAddr)> {
        nonblocking_future(|| self.0.recv_from(buf)).await
    }

    pub async fn peek(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<usize> {
        nonblocking_future(|| self.0.peek(buf)).await
    }

    pub async fn peek_from(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<(usize, SockAddr)> {
        nonblocking_future(|| self.0.peek_from(buf)).await
    }

    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_future(|| self.0.send(buf)).await
    }

    pub async fn send_to(&self, buf: &[u8], addr: &SockAddr) -> io::Result<usize> {
        nonblocking_future(|| self.0.send_to(buf, addr)).await
    }
}

pub fn nonblocking_future<T>(
    mut f: impl FnMut() -> io::Result<T>,
) -> impl Future<Output = io::Result<T>> {
    core::future::poll_fn(move |_| match f() {
        Ok(result) => Poll::Ready(Ok(result)),
        Err(err) if err.kind() == ErrorKind::WouldBlock => Poll::Pending,
        Err(err) if err.raw_os_error().is_some_and(|c| c == 36) => Poll::Pending,
        Err(err) => Poll::Ready(Err(err)),
    })
}
