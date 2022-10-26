use std::{task::{RawWakerVTable, RawWaker, Context}, future::Future, pin::Pin};

pub struct AnimationExecutor<F: Future<Output = ()>> {
    waker: std::task::Waker,
    future: Option<Pin<Box<F>>>
}

impl<F: Future<Output = ()>> AnimationExecutor<F> {
    pub fn new(animation: F) -> Self {
        Self {
            waker: Waker::new(),
            future: Some(Box::pin(animation)),
        }
    }

    pub fn frame(&mut self) -> bool {
        let mut future = self.future.take().unwrap();
        let mut context = Context::from_waker(&self.waker);
        match Future::poll(future.as_mut(), &mut context) {
            std::task::Poll::Ready(_) => { false },
            std::task::Poll::Pending => {
                self.future = Some(future);
                true
            },
        }
    }
}


// ----- waker -----
pub struct Waker;

impl Waker {
    pub fn new() -> std::task::Waker {
        unsafe { std::task::Waker::from_raw(RawWaker::new(0x0 as *const (), waker_vtable())) }
    }
}

pub fn waker_vtable() -> &'static RawWakerVTable {
    &RawWakerVTable::new(
        clone_raw,
        wake_raw,
        wake_by_ref_raw,
        drop_raw,
    )
}

unsafe fn clone_raw(_data: *const ()) -> RawWaker {
    RawWaker::new(0x0 as *const (), waker_vtable())
}

unsafe fn wake_raw(_data: *const ()) {}

unsafe fn wake_by_ref_raw(_data: *const ()) {}

unsafe fn drop_raw(_data: *const ()) {}

