use core::{ future::Future, pin::Pin };
use core::task::{ Context, Poll };
use alloc::boxed::Box;
use crate::std::syscall;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

impl TaskId {
	pub fn new() -> Self {
		TaskId(syscall::get_task_id())
	}
}

pub struct Task {
	pub id: TaskId,
	future: Pin<Box<dyn Future<Output = ()>>>
}

impl Task {
	pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
		Task {
			id: TaskId::new(),
			future: Box::pin(future),
		}
	}

	fn poll(&mut self, context: &mut Context) -> Poll<()> {
		self.future.as_mut().poll(context)
	}
}

pub struct YieldNow {
	yielded: bool
}

impl YieldNow {
	pub fn new() -> Self {
		Self { yielded: false }
	}
}

impl Future for YieldNow {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
		if self.yielded {
			Poll::Ready(())
		} else {
			self.yielded = true;
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	}
}

pub async fn cooperate() {
	YieldNow::new().await;
}
