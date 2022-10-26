use std::{pin::Pin, time::{Duration, Instant}};
use async_trait::async_trait;
use future::AnimationExecutor;

mod future;

trait AnimatableValue: Sized {}
impl<T: Sized> AnimatableValue for T {}

// #[async_trait(?Send)]
// impl<T> Animation<T> for T
// where
//     T: AnimatableValue,
// {
//     type Inner = T;
// 
//     fn get_inner(&self) -> &Self::Inner { self }
//     fn get_inner_mut(&mut self) -> &mut Self::Inner { self }
// 
//     async fn run(&mut self) { println!("test"); next_frame().await }
// }


#[async_trait(?Send)]
trait Animation {
    type Inner: Animation<Value = Self::Value>;
    type Value: AnimatableValue;

    // fn get_item(&self) -> &Self::Value { self.get_inner().get_item() }
    // fn get_item_mut(&mut self) -> &mut Self::Value { self.get_inner_mut().get_item_mut() }

    // fn get_inner(&self) -> &Self::Inner;
    // fn get_inner_mut(&mut self) -> &mut Self::Inner;

    async fn run(&mut self) -> &mut Self::Value;
}

trait IntoAnim: Sized {
    fn anim(&mut self) -> Anim<Self> {
        Anim { item: self }
    }
}

impl IntoAnim for f32 {}

struct Anim<'a, T> {
    item: &'a mut T,
}

#[async_trait(?Send)]
impl<'a, T: AnimatableValue> Animation for Anim<'a, T> {
    type Inner = Self;
    type Value = T;

    async fn run(&mut self) -> &mut Self::Value { self.item }
}

// --- tween animation ---
struct TweenAnimation<T, I, F>
where
    T: AnimatableValue,
    I: Animation<Value = T>,
    F: FnMut(T, T, f32) -> T,
{
    inner: I,
    target: T,
    tween_fn: F,
    duration: Duration,
}

#[async_trait(?Send)]
impl<T, I, F> Animation for TweenAnimation<T, I, F>
where
    T: AnimatableValue + Copy,
    I: Animation<Value = T>,
    F: FnMut(T, T, f32) -> T,
{
    type Inner = I;
    type Value = T;

    async fn run(&mut self) -> &mut Self::Value {
        let item = self.inner.run().await;
        let start_t = Instant::now();

        let start_value = *item;
        while start_t.elapsed() < self.duration {
            next_frame().await;
            let t = start_t.elapsed().as_secs_f32() / self.duration.as_secs_f32();

            *item = (self.tween_fn)(start_value, self.target, t);
        }
        *item = (self.tween_fn)(start_value, self.target, 1.0);

        item
    }
}

trait TweenAnimatable: Animation + Sized {
    fn tween<F>(self, target: Self::Value, tween_fn: F, duration: Duration) -> TweenAnimation<Self::Value, Self, F>
    where
        F: FnMut(Self::Value, Self::Value, f32) -> Self::Value,
    {
        TweenAnimation {
            inner: self,
            target,
            tween_fn,
            duration,
        }
    }
}
// --- tween animation ---

impl<T: Animation + Sized> TweenAnimatable for T {}

struct AnimationWrapper<T: Animation>(Option<T>);

impl<T: Animation> From<T> for AnimationWrapper<T> {
    fn from(v: T) -> Self {
        AnimationWrapper(Some(v))
    }
}

impl<T: Animation + 'static> std::future::IntoFuture for AnimationWrapper<T> {
    type Output = ();
    type IntoFuture = Pin<Box<dyn std::future::Future<Output = ()>>>;

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move { self.0.take().unwrap().run().await; })
    }
}


// not needed
struct WaitFuture {
    duration: Duration,
    start_t: Instant,
}

impl WaitFuture {
    fn new(duration: Duration) -> Self { Self { duration, start_t: Instant::now() } }
}

impl std::future::Future for WaitFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.duration > self.start_t.elapsed() { std::task::Poll::Pending }
        else { std::task::Poll::Ready(()) }
    }
}

struct NextFrameFuture {
    finished: bool,
}

impl NextFrameFuture {
    fn new() -> Self { Self { finished: false } }
}

impl std::future::Future for NextFrameFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if !self.finished {
            self.as_mut().finished = true;
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}

fn next_frame() -> NextFrameFuture { NextFrameFuture::new() }

fn wait(duration: Duration) -> WaitFuture { WaitFuture::new(duration) }

fn lerp<T: std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>>(a: T, b: T, t: f32) -> T {
    a * (1.0 - t) + b * t
}

fn main() {
    let fut = async {
        println!("----- anim -----");
        let mut v = 1.0f32;
        dbg!(v);
        v.anim()
            .tween(2.0, lerp, Duration::from_secs(1))
            .tween(3.0, lerp, Duration::from_secs(1))
            .run().await;

        dbg!(v);

        println!("waiting");
        wait(Duration::from_secs(1)).await;
        println!("finished waiting");

        v.anim()
            .tween(2.0, lerp, Duration::from_secs(1))
            .run().await;
        dbg!(v);
    };

    let mut executor = AnimationExecutor::new(fut);

    let mut current_frame = 0;
    let start_t = Instant::now();
    while executor.frame() {
        std::thread::sleep(Duration::from_millis(16));
        current_frame += 1;
    }
    println!("got {current_frame} frames in {}s", start_t.elapsed().as_secs_f32());
}

