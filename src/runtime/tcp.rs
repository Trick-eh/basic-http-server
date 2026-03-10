use std::{
    future::Future,
    io::{self, Read, Write},
    net::{TcpListener as StdListener, TcpStream as StdStream, ToSocketAddrs},
    os::unix::io::AsRawFd,
    pin::Pin,
    task::{Context, Poll},
};

use super::reactor::{Interest, reactor};

pub struct TcpListener {
    inner: StdListener,
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let listener = StdListener::bind(addr)?;

        listener.set_nonblocking(true)?;
        Ok(TcpListener { inner: listener })
    }

    pub fn accept(&self) -> AcceptFuture<'_> {
        AcceptFuture { listener: self }
    }
}

pub struct AcceptFuture<'a> {
    listener: &'a TcpListener,
}

impl<'a> Future for AcceptFuture<'a> {
    type Output = io::Result<TcpStream>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.listener.inner.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(true)?;
                Poll::Ready(Ok(TcpStream { inner: stream }))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                reactor().register(
                    self.listener.inner.as_raw_fd(),
                    Interest::Readable,
                    cx.waker().clone(),
                );
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

pub struct TcpStream {
    inner: StdStream,
}

impl TcpStream {
    pub fn read<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadFuture {
        ReadFuture { stream: self, buf }
    }

    pub fn write_all<'a>(&'a mut self, data: &'a [u8]) -> WriteFuture {
        WriteFuture {
            stream: self,
            data,
            written: 0,
        }
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        reactor().deregister(self.inner.as_raw_fd());
    }
}

pub struct ReadFuture<'a> {
    stream: &'a mut TcpStream,
    buf: &'a mut Vec<u8>,
}

impl<'a> Future for ReadFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { stream, buf } = &mut *self;
        match stream.inner.read(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                reactor().register(
                    stream.inner.as_raw_fd(),
                    Interest::Readable,
                    cx.waker().clone(),
                );
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

pub struct WriteFuture<'a> {
    stream: &'a mut TcpStream,
    data: &'a [u8],
    written: usize,
}

impl<'a> Future for WriteFuture<'a> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            stream,
            data,
            written,
        } = &mut *self;

        loop {
            match stream.inner.write(&data[*written..]) {
                Ok(n) => {
                    *written += n;
                    if *written == data.len() {
                        return Poll::Ready(Ok(()));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    reactor().register(
                        stream.inner.as_raw_fd(),
                        Interest::Writable,
                        cx.waker().clone(),
                    );
                    return Poll::Pending;
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}
