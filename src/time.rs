use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub struct Sleep(Instant);

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() > self.0 {
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

pub fn sleep(duration: Duration) -> Sleep {
    let since = Instant::now().checked_add(duration).unwrap();

    Sleep(since)
}
