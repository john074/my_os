use core::{ future::Future, pin::Pin };
use core::sync::atomic::{ AtomicU64, Ordering };
use core::task::{ Context, Poll, Waker };
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::task::Wake;
use crossbeam_queue::ArrayQueue;

use crate::cpu;

pub static mut EXECUTOR_PTR: *mut Executor = core::ptr::null_mut();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

impl TaskId {
	pub fn new() -> Self {
		static NEXT_ID: AtomicU64 = AtomicU64::new(0);
		TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
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

pub struct Executor {
	pub tasks: BTreeMap<TaskId, Task>,
	pub task_queue: Arc<ArrayQueue<TaskId>>,
	waker_cache: BTreeMap<TaskId, Waker>,
	pub current_task: Option<TaskId>,
}

impl Executor {
	pub fn new() -> Executor {
		Executor {
			tasks: BTreeMap::new(),
			task_queue: Arc::new(ArrayQueue::new(100)),
			waker_cache: BTreeMap::new(),
			current_task: None
		}
	}

	pub fn spawn(&mut self, task: Task) {
		let task_id = task.id;
		if self.tasks.insert(task.id, task).is_some() {
			panic!("Task id is taken");
		}
		self.task_queue.push(task_id).expect("queue is full");
	}

	fn run_ready_tasks(&mut self) {
		cpu::enable_interrupts();
		let Self {
			tasks,
			task_queue,
			waker_cache,
			current_task,
		} = self;
		while let Some(task_id) = task_queue.pop() {
			//println!("{:#?}", task_id);
			let task = match tasks.get_mut(&task_id) {
				Some(task) => task,
				None => continue
			};
			let waker = waker_cache.entry(task_id).or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
			let mut context = Context::from_waker(waker);
			self.current_task = Some(task_id);
			match task.poll(&mut context) {
				Poll::Ready(()) => {
					tasks.remove(&task_id);
					waker_cache.remove(&task_id);
				}
				Poll::Pending => {}
			}
			self.current_task = None;
		}
	}

	pub fn run(&mut self) -> ! {
		loop {
			self.run_ready_tasks();
			self.sleep_if_idle();
		}
	}

	fn sleep_if_idle(&self) {
		cpu::disable_interrupts();
		if self.task_queue.is_empty() {
			cpu::enable_interrupts();
			cpu::hlt();
		} else {
			cpu::enable_interrupts();	
		}
	}
}

struct TaskWaker {
	task_id: TaskId,
	task_queue: Arc<ArrayQueue<TaskId>>
}

impl TaskWaker {
	fn wake_task(&self) {
		self.task_queue.push(self.task_id).expect("task_queue is full");
	}
}

impl TaskWaker {
	fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
		Waker::from(Arc::new(TaskWaker {
			task_id,
			task_queue
		}))
	}
}

impl Wake for TaskWaker {
	fn wake(self: Arc<Self>) {
		self.wake_task();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		self.wake_task();
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
