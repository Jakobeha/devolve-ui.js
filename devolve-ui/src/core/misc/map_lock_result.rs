// TODO: Make this into its own crate and publish on Cargo?

use std::sync::{PoisonError, TryLockError};

pub trait MappableLockError<A, B> {
    type SelfB;

    fn map(self, transform: impl FnOnce(A) -> B) -> Self::SelfB;
}

pub trait MappableLockResult<A, B> {
    type SelfB;

    fn map2(self, transform: impl FnOnce(A) -> B) -> Self::SelfB;
}

impl <A, B> MappableLockError<A, B> for PoisonError<A> {
    type SelfB = PoisonError<B>;

    fn map(self, transform: impl FnOnce(A) -> B) -> Self::SelfB {
        PoisonError::new(transform(self.into_inner()))
    }
}


impl <A, B> MappableLockError<A, B> for TryLockError<A> {
    type SelfB = TryLockError<B>;

    fn map(self, transform: impl FnOnce(A) -> B) -> Self::SelfB {
        match self {
            TryLockError::Poisoned(err) => TryLockError::Poisoned(err.map(transform)),
            TryLockError::WouldBlock => TryLockError::WouldBlock
        }
    }
}

impl <A, B, E: MappableLockError<A, B>> MappableLockResult<A, B> for Result<A, E> {
    type SelfB = Result<B, E::SelfB>;

    fn map2(self, transform: impl FnOnce(A) -> B) -> Self::SelfB {
        match self {
            Ok(result) => Ok(transform(result)),
            Err(err) => Err(err.map(transform))
        }
    }
}