use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};

pub enum EitherFuture<Left: Future<Output=Output>, Right: Future<Output=Output>, Output> {
    Left(Left),
    Right(Right)
}

impl<Left: Future<Output=Output>, Right: Future<Output=Output>, Output> Future for EitherFuture<Left, Right, Output> {
    type Output = Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match unsafe { self.get_unchecked_mut() } {
            EitherFuture::Left(f) => unsafe { Pin::new_unchecked(f) }.poll(cx),
            EitherFuture::Right(f) => unsafe { Pin::new_unchecked(f) }.poll(cx)
        }
    }
}