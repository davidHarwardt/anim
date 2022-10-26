# a rust animation engine (like motion-canvas or manim)


## components
- animatable value


### animatable value
```rust
struct Scene {
    resolution: (usize, usize),
    view_rect: Rect,
    // ...
}

// used with scene.goto_scene(<target>: SceneTarget)
enum SceneTarget {
    Next,
    NthNext(usize),
    Prev,
    NthPrev(usize),

    Nth(usize),
    NthFromEnd(usize),
    First,
    Last,

    SceneWithId(String),
}

enum FileFormat {
    Mp4,
    PngSequence,
}

async fn test_scene(scene: &Scene) {
    let mut circle = Circle::new((0, 0));
    let mut rect = Rect::new((-1, 0));
    scene.add(&circle);
    scene.add(&rect);

    // simply animate a property over the specified time
    circle.pos.to(pos2(1, 0), 2.0.seconds()).await;

    // start the animation delayed
    circle.pos
        .wait(0.5.seconds())
        // chain multiple animations together
        .to(pos(0, -1), 1.5.seconds()).await;

    // run multiple animations in paralell and wait until all of them are finished
    all![
        circle.color.to(Color::ORANGE, 1.5.seconds()),
        circle.radius.to(2.0, 1.0.seconds()),
    ].await;

    // run animation async (returns a joinhandle to await the animations later)
    scene.run_async(all![
        circle.color
            .to(Color::BLUE, 1.5.seconds())
            .to(Color::RED, 1.5.seconds())
            .loop(),
    ]);

    // specify the tweening function for the animation
    rect.pos.tween(pos2(-1, 1), tween_function::ease_out, 2.0.seconds()).await;
    // or use a custom one
    rect.pos.tween(pos2(-1, -1), |a, b, v| {
        a * (1 - v) + b * v
    }, 2.0.seconds()).await;

    // delay the code executing next by the time specified
    scene.delay(2.0.seconds()).await;

    // only advance 1 frame
    // optionaly get deltatime from last frame
    let dt = next_frame().await;
    rect.pos.set(pos2(0, 0));

    // await events like Marker, Next, ...
    scene.event(Event::Marker("test")).await;
}

fn main() {
    let scene = SceneBuilder::new()
        .with_resolution(1920, 1080)
        .with_fps(60)

        .with_window(true),
        // name of the output file determined by commandline args
        .with_output_file(&OutputFileOptions {
            format: FileFormat::Mp4,
        })
        .with_scenes(&[
            ("test_scene", test_scene),
            // or
            named_scene!(test_scene),
        ]).build();

    scene.run();
}
```


### impl notes

```rust

trait AnimatableValue: Animatable {
    type Item = Self;
    type Inner = Self;

    fn get_item(&self) -> &Self::Item { self }
    fn get_item_mut(&mut self) -> &mut Self::Item { self }

    async fn run(&mut self) {}
}

trait Animatable {
    // todo: look at correctness
    type Item: AnimatableValue = Self::Inner::Item;
    type Inner: Animatable;

    fn tween(self, target: Self::Item, transition_fn: impl FnMut(Self::Item, Self::Item, f32), duration: Duration) -> TweenAnimation<Self> {
        TweenAnimation {
            inner: self,
            target,
            duration,
            transition_fn,
        }
    }

    fn get_item(&self) -> &Self::Item { self.get_inner().get_item() }
    fn get_item_mut(&mut self) -> &mut Self::Item { self.get_inner_mut().get_item_mut() }

    
    fn get_inner(&self) -> &Self::Inner;
    fn get_inner_mut(&mut self) -> &mut Self::Inner;

    async fn run(&mut self);
}

impl std::future::IntoFuture for Animatable {
    type Output = ();
    type IntoFuture = impl Future<Output = ()>;
    
    fn into_future(self) -> Self::IntoFuture { self.run() }
}

// implement into future for all animations to run them on await

struct TweenAnimation<T, U, F> {
    inner: T,
    target: U,
    duration: Duration,
    transition_fn: F,
}

impl<T, F> Animatable for TweenAnimation<T, Self::Item, F>
where
    T: Animatable,
    F: FnMut(Self::Item, Self::Item, f32),
{
    type Inner = T;

    async fn run(&mut self) {
        self.inner.run().await;

        let start = Instant::now();
        let item = self.inner.get_item_mut();
        let start_value = *item;
        while start.elapsed() < self.duration {
            next_frame().await;
            let t = start.elapsed().div_duration_f32(self.duration);

            *item = (self.transition_fn)(start_value, self.target, t);
        }

        *item = (self.transition_fn)(start_value, self.target, 1.0);
    }
}
```









































































### syntax for scene components
```rust
fn test_scene(scene: &mut Scene) {
    scene.add();
  
    scene.add(v![
        HStack {
        Icon(IconType::Logo);
        Text("test", color = Color::BLUE)
            .foreground_color(Color::BLUE);
        };
    ]);
    // expands to
    scene.add(HStack::new()
        .with_child(Icon::new(IconType::Logo))
        .with_child(Text::new("test").foreground_color(Color::BLUE))
    );
}
```

```rust
fn scene(scene: &mut Scene) {
    let text = v![Text("test")];
    let test = v![Surface(border_radius: 2.0) {
        HStack {
            text;
        };
    }];

    let other = v![ForEach(<items>, |item| v![
        Text(format!("{}", item)),
    ])];

    v![<Name>(constructor_args, named_args: <value>) {
        <children>;
    }];
    // expands to
    let text = Text::new("test")
        .width(50.percent())
        .finish();
    let test = Surface::new()
        .add_children(vec![
            Box::new(HStack::new()
                .add_children(vec![
                    Box::new(text.finish()),
                ])
                .finish()),
        ])
        .border_radius(2.0)
        .finish();
}

trait SceneItemBuilder {
    type Item: SceneItem;
    
    // new method required
    
    fn finish() -> Self::Item { /* userimplemented */ }

    fn add_children(children: Vec<Box<dyn SceneItem>>) -> Self {}
}

trait SceneItem {
    fn insert(&self) -> Box<dyn SceneItem>;
}

struct TestItem {
    test: bool,

    other: Animatable<f32>,
}

impl TestItem {
    fn new() -> TestItemBuilder {

    }
}
```


```rust
fn test_scene(scene: &mut Scene) {
    let text = v![Text("test", color: Color::RED)];

    scene.add(v![
        Surface(border_radius: 2.0) {
            HStack {
                text;
            }
        }
    ]);
    // gets transformed to
    scene.add(
        Surface::new()
    );

    // run an animation and wait until it is finished
    scene.animate(text.color.to(Color::GREEN, 0.5));

    // wait for an animation event => Next in presentation (shorthand for scene.animate(<WaitForAnimation>))
    scene.wait_for(Event::Next);
    // wait for some time (shorthand for scene.animate(<WaitAnimation>))
    scene.wait(0.5);

    // run animation in parallel and wait until every animation is finished
    scene.animate(all![
        text.color.to(Color::GREEN, 0.5),
        // add a delay to the animation
        text.size.to(20.0, 0.25).delay(0.25),
    ]);

    // run an animation in the background (async)
    scene.animate_async(text.color.to(Color::LIME, 0.25));
    scene.animate(text.size.to(4.0, 5.0)); // <== animation runs immediately after
  
    // async animations get canceled
}
```


```rust
// types can implement
trait AnimatableValue<T> {
  fn to(&mut self, value: T, duration: Into<Duration>) -> TweenAnimation<T> {
    
  }
  
  fn tween(&mut self, func: impl Fn(v: f32) -> T, duration: Into<Duration>) -> TweenAnimation<T> {
    // tween animation has option to specify easing
  }
}

impl AnimatedValue<T> {
    
}

impl Scene {
  fn add(objects: impl SceneObject) {
    
  }
  
  fn animate(anim: impl Animation) {
    
  }
  
  fn animate_async(anim: impl Animation) {
    
  }
}
```

