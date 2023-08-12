

Both [`Singleton`] and [`Transient`] are attribute macros used to define a [`Provider`], the difference between them is that a `Provider` defined by `#[Singleton]` has a constructor method that is executed only once, while a `Provider` defined by `#[Transient]` has its constructor method can be executed multiple times.

These two macros can be used on `struct`, `impl struct`, and `fn`.

- When used on `struct` and `impl struct`, a [`DefaultProvider`] implementation is generated for the struct, and the associated type [`DefaultProvider::Type`] is the struct itself.

- When used on `fn`, a struct with the same name as the function is created, and then a [`DefaultProvider`] implementation is generated for the struct, with the associated type [`DefaultProvider::Type`] being the return type of the function. As mentioned above, it is recommended to use `CamelCase` when defining functions. Of course, you can still use `snake_case`.

## Example

```rust
use rudi::{Context, Singleton, Transient};

#[derive(Clone)]
struct A;

#[Singleton(name = "a")]
impl A {
    fn new() -> Self {
        Self
    }
}

#[Singleton(name = "b")]
#[derive(Clone)]
struct B(#[di(name = "a")] A);

#[Transient]
fn C(#[di(name = "b")] b: B) -> i32 {
    let _ = b;
    42
}

fn main() {
    let mut cx = Context::auto_register();
    let number = cx.resolve::<i32>();
    println!("number = {}", number);
}
```

## Customization with attributes

### On `struct` and `function`

#### Generic attributes that can be used on `struct`, `impl struct`, and `fn`

- name
  - type: any expression that implements `Into<Cow<'static, str>>`.
  - example: `#[Singleton(name = "abc")]` / `#[Transient(name = a::b::NAME)]` / `#[Transient(name = nth(42))]`
  - optional: true
  - default: ""
  - description: Specifies the name of the defined `Provider`.
  - refer:
    - [`SingletonProvider::name`]
    - [`TransientProvider::name`]
    - [`SingletonAsyncProvider::name`]
    - [`TransientAsyncProvider::name`]

- eager_create
  - type: bool
  - example: `#[Singleton(eager_create)]`
  - optional: true
  - default: false
  - description: Specifies whether the defined `Provider` is eager created.
  - refer:
    - [`SingletonProvider::eager_create`]
    - [`TransientProvider::eager_create`]
    - [`SingletonAsyncProvider::eager_create`]
    - [`TransientAsyncProvider::eager_create`]

- binds
  - type: Array of paths to functions of type `fn(T) -> R`, where `T` is current struct type or current function return type and `R` can be any type.
  - example: `#[Singleton(binds = [Rc::new, Box::new])]`
  - optional: true
  - default: None
  - description: Specifies the field `binding_providers` and `binding_definitions` of the defined `Provider`.
  - refer:
    - [`SingletonProvider::bind`]
    - [`TransientProvider::bind`]
    - [`SingletonAsyncProvider::bind`]
    - [`TransientAsyncProvider::bind`]

- not_auto_register
  - type: bool
  - example: `#[Singleton(not_auto_register)]`
  - optional: true
  - default: false
  - description: Specifies whether a defined `Provider` should be auto-registered to [`AutoRegisterModule`](crate::AutoRegisterModule). When the `auto-register` feature is enabled (which is enabled by default), this attribute can be used if auto-registration is not desired, or if auto-registration is not possible due to the presence of generics.

#### An attribute that can only be used on `struct`

- async_constructor
  - type: bool
  - example: `#[Singleton(async_constructor)]`
  - optional: true
  - default: false
  - description: Specifies whether the constructor method of a defined `Provider` is asynchronous. Only valid when used on `struct`, for `impl struct` and `fn` cases use `async fn`.

### On `field` of struct and `argument` of function

When adding attributes to `field` of struct and `argument` of function, you need to use `#[di(...)] `.

- name
  - conflict: `vector`
  - type: any expression that implements `Into<Cow<'static, str>>`.
  - example: `#[di(name = "abc")]` / `#[di(name = a::b::NAME)]` / `#[di(name = nth(42))]`
  - optional: true
  - default: ""
  - description: Specifies the name of the dependency to be taken out of `Context`.
  - refer:
    - [`Context::resolve_with_name`]
    - [`Context::resolve_with_name_async`]

- option
  - conflict: `default`, `vector`
  - require: The current `field` or `argument`, which must be of type [`Option<T>`].
  - type: `T`.
  - example: `#[di(option = i32)]` / `#[di(option = String)]`
  - optional: true
  - default: None
  - description:

    From the call to the following method
    - `cx.resolve_with_name::<Option<T>>(name)`
    - `cx.resolve_with_name_async::<Option<T>>(name).await`

    Instead, call the following method
    - `cx.resolve_option_with_name::<T>(name)`
    - `cx.resolve_option_with_name_async::<T>(name).await`

  - refer:
    - [`Context::resolve_option_with_name`]
    - [`Context::resolve_option_with_name_async`]

- default
  - conflict: `option`, `vector`
  - require: If no default value is specified, the current `field` or `argument` must implement the [`Default`] trait.
  - type: empty, or an arbitrary expression type.
  - example: `#[di(default)]` / `#[di(default = 42)]` / `#[di(default = a::b::func())]`
  - optional: true
  - default: None
  - description:

    From the call to the following method
    - `cx.resolve_with_name(name)`
    - `cx.resolve_with_name_async(name).await`

    Instead, call the following method
    - `match cx.resolve_option_with_name(name) { ... }`
    - `match cx.resolve_option_with_name_async(name).await { ... }`

  - refer:
    - [`Context::resolve_option_with_name`]
    - [`Context::resolve_option_with_name_async`]

- vector
  - conflict: `name`, `option`, `default`
  - require: The current `field` or `argument`, which must be of type [`Vec<T>`].
  - type: `T`.
  - example: `#[di(vector = i32)]` / `#[di(vector = String)]`
  - optional: true
  - default: None
  - description:

    From the call to the following method
    - `cx.resolve_with_name::<Vec<T>>(name)`
    - `cx.resolve_with_name_async::<Vec<T>>(name).await`

    Instead, call the following method
    - `cx.resolve_by_type::<T>()`
    - `cx.resolve_by_type_async::<T>()`

  - refer:
    - [`Context::resolve_by_type`]
    - [`Context::resolve_by_type_async`]

## Struct or function attributes examples

```rust
use std::{borrow::Cow, fmt::Debug, marker::PhantomData, rc::Rc};

use rudi::{components, modules, AutoRegisterModule, Context, Module, Singleton, Transient};

const NAME_A: &str = "a";

const fn name_b() -> &'static str {
    "b"
}

fn name_c() -> impl Into<Cow<'static, str>> {
    "c"
}

fn transform<T: Debug + 'static>(t: T) -> Rc<dyn Debug> {
    Rc::new(t)
}

#[derive(Clone, Debug)]
#[Singleton(name = NAME_A, eager_create)]
struct NameAndEagerCreate;

#[derive(Debug)]
#[Transient(name = name_b(), binds = [transform])]
struct NameAndBinds;

#[Transient(name = name_c())]
async fn AsyncDep() -> i32 {
    42
}

#[derive(Debug)]
#[Transient(async_constructor)]
struct Async(#[di(name = name_c())] i32);

#[Transient(not_auto_register)]
async fn NotAutoRegister<T: Debug + 'static>(#[di(name = name_c())] t: T) -> T {
    t
}

#[Singleton(not_auto_register)]
async fn Run<T: Debug + 'static>(
    #[di(name = NAME_A)] _name_and_eager_create: NameAndEagerCreate,
    #[di(name = name_b())] name_and_binds: NameAndBinds,
    #[di(name = name_b())] dyn_debug: Rc<dyn Debug>,
    async_: Async,
    not_auto_register: T,
) {
    assert_eq!(format!("{:?}", name_and_binds), format!("{:?}", dyn_debug));
    assert_eq!(async_.0, 42);
    println!("not_auto_register: {:?}", not_auto_register);
}

struct MyModule<T>(PhantomData<T>);

impl<T: Debug + 'static> Module for MyModule<T> {
    fn providers() -> Vec<rudi::DynProvider> {
        components![NotAutoRegister<T>, Run<T>]
    }
}

#[tokio::main]
async fn main() {
    let mut cx = Context::create(modules![AutoRegisterModule, MyModule<i32>]);

    cx.resolve_async().await
}
```

## Field or argument attributes examples

Although the following example only shows how to use attributes on `field`, it is the same as using them on `argument`.

```rust
use rudi::{Context, Singleton, Transient};

// name

#[Singleton]
fn One() -> i8 {
    1
}

#[Singleton(name = "2")]
fn Two() -> i8 {
    2
}

#[Transient]
struct A(i8);

#[Transient]
struct B(#[di(name = "2")] i8);

// option

#[Singleton]
fn Three() -> Option<i16> {
    Some(3)
}

#[Singleton]
fn Four() -> i16 {
    4
}

#[Transient]
struct C(Option<i16>);

#[Transient]
struct D(#[di(option = i16)] Option<i16>);

// default

#[Transient]
struct E(#[di(default)] i32);

#[Transient]
struct F(#[di(default = 42)] i32);

// vector

#[Singleton]
fn Five() -> Vec<i64> {
    vec![5]
}

#[Singleton]
fn Six() -> i64 {
    6
}

#[Transient]
struct G(Vec<i64>);

#[Transient]
struct H(#[di(vector = i64)] Vec<i64>);

#[Singleton]
fn Run(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: G) {
    assert_eq!(a.0, 1);
    assert_eq!(b.0, 2);

    assert_eq!(c.0.unwrap(), 3);
    assert_eq!(d.0.unwrap(), 4);

    assert_eq!(e.0, 0);
    assert_eq!(f.0, 42);

    assert_eq!(g.0[0], 5);
    assert_eq!(h.0[0], 5);
}

fn main() {
    let mut cx = Context::auto_register();
    cx.resolve()
}
```
