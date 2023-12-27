[`#[Singleton]`](crate::Singleton), [`#[Transient]`](crate::Transient) and [`#[SingleOwner]`](crate::SingleOwner) are attribute macros used to define a [`Provider`], for the difference between the `Provider`s they defined, see [`Scope`].

These three macros can be used on `struct`, `enum`, `impl block`, and `fn`.

- When used on `struct`, `enum` and `impl block`, a [`DefaultProvider`] implementation is generated for the `struct` or `enum`, and the associated type [`DefaultProvider::Type`] is the `struct` or `enum` itself.

- When used on `fn`, a struct with the same name as the function is created, and then a [`DefaultProvider`] implementation is generated for the struct, with the associated type [`DefaultProvider::Type`] being the return type of the function. As mentioned above, it is recommended to use `CamelCase` when defining functions. Of course, you can still use `snake_case`.

## Example

```rust
use rudi::{Context, Singleton, Transient};

// impl block

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct A;

#[Singleton]
impl A {
    fn new() -> Self {
        Self
    }
}

// struct

#[Singleton]
#[derive(Debug, Clone, PartialEq, Eq)]
struct B(A);

// enum

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
#[Transient]
enum C {
    One,

    Two(A),

    #[di]
    Three {
        b: B,
    },
}

// fn

#[Singleton]
fn Run(a: A, b: B, c: C) {
    assert_eq!(b, B(a));
    assert_eq!(c, C::Three { b: B(a) });
}

fn main() {
    let mut cx = Context::auto_register();
    cx.resolve()
}
```

## Attribute arguments

### `#[Singleton]` / `#[Transient]` / `#[SingleOwner]`: used on `struct`, `enum`, `impl block` and `fn`

#### Common arguments that can be used on `struct`, `enum`, `impl block`, and `fn`

- name
  - type: any expression that implements `Into<Cow<'static, str>>`.
  - example: `#[Singleton(name = "abc")]` / `#[Transient(name = a::b::NAME)]` / `#[SingleOwner(name = nth(42))]`
  - optional: true
  - default: **""**
  - description: Specifies the name of the defined `Provider`.
  - refer:
    - [`SingletonProvider::name`]
    - [`TransientProvider::name`]
    - [`SingleOwnerProvider::name`]
    - [`SingletonAsyncProvider::name`]
    - [`TransientAsyncProvider::name`]
    - [`SingleOwnerAsyncProvider::name`]

- eager_create
  - type: bool
  - example: `#[Singleton(eager_create)]` / `#[Transient(eager_create = true)]` / `#[SingleOwner(eager_create = false)]`
  - optional: true
  - default: **false**
  - description: Specifies whether the defined `Provider` is eager to create.
  - refer:
    - [`SingletonProvider::eager_create`]
    - [`TransientProvider::eager_create`]
    - [`SingleOwnerProvider::eager_create`]
    - [`SingletonAsyncProvider::eager_create`]
    - [`TransientAsyncProvider::eager_create`]
    - [`SingleOwnerAsyncProvider::eager_create`]

- condition
  - type: a closure or an expression path of type `fn(&Context) -> bool`.
  - example: `#[Singleton(condition = |_cx| true)]` / `#[SingleOwner(condition = path::to::expr)]`
  - optional: true
  - default: **None**
  - description: Specifies whether or not to insert the defined `Provider` into the `Context` based on the condition.
  - refer:
    - [`SingletonProvider::condition`]
    - [`TransientProvider::condition`]
    - [`SingleOwnerProvider::condition`]
    - [`SingletonAsyncProvider::condition`]
    - [`TransientAsyncProvider::condition`]
    - [`SingleOwnerAsyncProvider::condition`]

- binds
  - type: Array of paths to functions of type `fn(T) -> R`, where `T` is current struct type or current function return type and `R` can be any type.
  - example: `#[Singleton(binds = [Rc::new, Box::new])]`
  - optional: true
  - default: **None**
  - description: Specifies the field `binding_providers` and `binding_definitions` of the defined `Provider`.
  - refer:
    - [`SingletonProvider::bind`]
    - [`TransientProvider::bind`]
    - [`SingleOwnerProvider::bind`]
    - [`SingletonAsyncProvider::bind`]
    - [`TransientAsyncProvider::bind`]
    - [`SingleOwnerAsyncProvider::bind`]

- auto_register
  - **available only when the `auto-register` feature flag is enabled**
  - type: bool
  - example: `#[Singleton(auto_register)]` / `#[Transient(auto_register = true)]` / `#[SingleOwner(auto_register = false)]`
  - optional: true
  - default: **true**
  - description: Specifies whether a defined `Provider` should be auto-registered to [`AutoRegisterModule`](crate::AutoRegisterModule). When the `auto-register` feature is enabled (which is enabled by default), this argument can be used if auto-registration is not desired, or if auto-registration is not possible due to the presence of generics.

#### An argument that can only be used on `struct` and `enum`

- async
  - type: bool
  - example: `#[Singleton(async)]`
  - optional: true
  - default: **false**
  - description: Specifies whether the constructor method of a defined `Provider` is asynchronous. Only valid when used on `struct` and `enum`, for `impl block` and `fn` cases use `async fn`.

### `#[di]`: used on `struct`, `enum`, `impl block` and `fn`

- rudi_path
  - type: path to the `rudi` crate.
  - example: `#[di(rudi_path = path::to::rudi)]`
  - optional: true
  - default: **::rudi**
  - description: Specifies the path to the `rudi` crate. This argument is used when the `rudi` crate is not in the root of the crate.

### `#[di]`: used on `variant` of enum

Use `#[di]` to specify which variant of the enum will be constructed.

### `#[di]`: used on `field` of struct, `field` of variant of enum and `argument` of function

- name
  - conflict: `vec`
  - type: any expression that implements `Into<Cow<'static, str>>`.
  - example: `#[di(name = "abc")]` / `#[di(name = a::b::NAME)]` / `#[di(name = nth(42))]`
  - optional: true
  - default: **""**
  - description: Specifies the name of the dependency to be taken out of `Context`.
  - refer:
    - [`Context::resolve_with_name`]
    - [`Context::resolve_with_name_async`]

- option
  - conflict: `default`, `vec`
  - require: The current `field` or `argument`, which must be of type [`Option<T>`].
  - type: bool.
  - example: `#[di(option)]`
  - optional: true
  - default: **false**
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
  - conflict: `option`, `vec`
  - require: If no default value is specified, the current `field` or `argument` must implement the [`Default`] trait.
  - type: empty, or an arbitrary expression type.
  - example: `#[di(default)]` / `#[di(default = 42)]` / `#[di(default = a::b::func())]`
  - optional: true
  - default: **None**
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

- vec
  - conflict: `name`, `option`, `default`
  - require: The current `field` or `argument`, which must be of type [`Vec<T>`].
  - type: bool.
  - example: `#[di(vec)]`
  - optional: true
  - default: **false**
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

- ref
  - require:
    - exist `option` argument: The current `field` or `argument`, which must be of type [`Option<&T>`].
    - exist `vec` argument: The current `field` or `argument`, which must be of type [`Vec<&T>`].
    - exist `default` argument or not, the current `field` or `argument`, which must be of type `&T`.
    - if using a type alias, specify the original type using `#[di(ref = T)]`, where `T` is a non-reference type.
  - type: `Option<Type>`
  - example:
    - `#[di(ref)]`
    - `#[di(ref = i32)]`
    - `#[di(option, ref)]`
    - `#[di(option, ref = i32)]`
    - `#[di(vec, ref)]`
    - `#[di(vec, ref = i32)]`
    - `#[di(default, ref)]`
    - `#[di(default, ref = i32)]`
    - `#[di(default = &42, ref)]`
    - `#[di(default = &42, ref = i32)]`
  - optional: true
  - default: **None**
  - description:

    Get a reference to `Singleton` or `SingleOwner` from `Context` .

    1. Not exist `option`, `vec` and `default` argument, will call the following method

        ```rust ignore
        // async
        cx.just_create_single_with_name_async::<T>(name).await;
        let var = cx.get_single_with_name(name);

        // sync
        cx.just_create_single_with_name::<T>(name);
        let var = cx.get_single_with_name(name);
        ```

    2. Exist `option` argument, will call the following method

        ```rust ignore
        // async
        cx.try_just_create_single_with_name_async::<T>(name).await;
        let var = cx.get_single_option_with_name(name);

        // sync
        cx.try_just_create_single_with_name::<T>(name);
        let var = cx.get_single_option_with_name(name);
        ```

    3. Exist `vec` argument, will call the following method

        ```rust ignore
        // async
        cx.try_just_create_singles_by_type_async::<T>().await;
        let var = cx.get_singles_by_type();

        // sync
        cx.try_just_create_singles_by_type::<T>();
        let var = cx.get_singles_by_type();
        ```

    4. Exist `default` argument, will call the following method

        ```rust ignore
        // async
        cx.try_just_create_single_with_name_async::<T>(name).await;
        let var = match cx.get_single_option_with_name(name) {
            Some(value) => value,
            None => default,
        };

        // sync
        cx.try_just_create_single_with_name::<T>(name);
        let var = match cx.get_single_option_with_name(name) {
            Some(value) => value,
            None => default,
        };
        ```

    5. If specified using `#[di(ref = R)]`, then all of the above `T`s will be replaced with the specified type `R`.

  - refer:
    - [`Context::just_create_single_with_name_async`]
    - [`Context::just_create_single_with_name`]
    - [`Context::try_just_create_single_with_name_async`]
    - [`Context::try_just_create_single_with_name`]
    - [`Context::try_just_create_singles_by_type_async`]
    - [`Context::try_just_create_singles_by_type`]
    - [`Context::get_single_with_name`]
    - [`Context::get_single_option_with_name`]
    - [`Context::get_singles_by_type`]

## Struct, enum and function attributes example

```rust
use std::{fmt::Debug, rc::Rc};

use rudi::{Context, Singleton, Transient};

// name

#[Transient]
fn One() -> i8 {
    1
}

#[Transient(name = "2")]
fn Two() -> i8 {
    2
}

// eager_create

#[Singleton(name = "3")]
fn Three() -> i16 {
    3
}

#[Singleton(name = "4", eager_create)]
fn Four() -> i16 {
    4
}

// condition

fn _5_condition(cx: &Context) -> bool {
    !cx.contains_single_with_name::<i32>("5")
}

#[Singleton(name = "5", condition = _5_condition)]
fn Five() -> i32 {
    5
}

#[Singleton(name = "6", condition = |_cx| false)]
fn Six() -> i32 {
    6
}

// binds

fn transform<T: Debug + 'static>(t: T) -> Rc<dyn Debug> {
    Rc::new(t)
}

#[Singleton(name = "7")]
fn Seven() -> i64 {
    7
}

#[Singleton(name = "8", binds = [transform])]
fn Eight() -> i64 {
    8
}

// auto_register

#[Singleton(name = "9")]
fn Nine() -> i128 {
    9
}

#[Singleton(name = "10", auto_register = false)]
fn Ten() -> i128 {
    10
}

// async

#[Transient]
struct A;

#[Transient(async)]
struct B;

// rudi_path

mod a {
    pub use rudi::*;
}

#[Transient]
#[di(rudi_path = rudi)]
struct C;

#[Transient]
#[di(rudi_path = a)]
struct D;

// `#[di]` used on `variant` of enum

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
#[Transient]
enum EF {
    #[di]
    E,
    F,
}

#[tokio::main]
async fn main() {
    let mut cx = Context::auto_register();

    assert_eq!(cx.resolve::<i8>(), 1);
    assert_eq!(cx.resolve_with_name::<i8>("2"), 2);

    assert!(!cx.contains_single_with_name::<i16>("3"));
    assert!(cx.contains_single_with_name::<i16>("4"));

    assert!(cx.contains_provider_with_name::<i32>("5"));
    assert!(!cx.contains_provider_with_name::<i32>("6"));

    assert!(cx.resolve_option_with_name::<Rc<dyn Debug>>("7").is_none());
    assert!(cx.resolve_option_with_name::<Rc<dyn Debug>>("8").is_some());

    assert!(cx.get_provider_with_name::<i128>("9").is_some());
    assert!(cx.get_provider_with_name::<i128>("10").is_none());

    assert!(cx.resolve_option::<A>().is_some());
    assert!(cx.resolve_option_async::<B>().await.is_some());

    assert!(cx.resolve_option::<C>().is_some());
    assert!(cx.resolve_option::<D>().is_some());

    assert_eq!(cx.resolve::<EF>(), EF::E);
}
```

## Field and argument attributes example

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
struct D(#[di(option)] Option<i16>);

// default

#[Transient]
struct E(#[di(default)] i32);

#[Transient]
struct F(#[di(default = 42)] i32);

// vec

#[Singleton]
fn Five() -> Vec<i64> {
    vec![5]
}

#[Singleton(eager_create)]
fn Six() -> i64 {
    6
}

#[Transient]
struct G(Vec<i64>);

#[Transient]
struct H(#[di(vec)] Vec<i64>);

#[Singleton]
fn Run(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H) {
    assert_eq!(a.0, 1);
    assert_eq!(b.0, 2);

    assert_eq!(c.0.unwrap(), 3);
    assert_eq!(d.0.unwrap(), 4);

    assert_eq!(e.0, 0);
    assert_eq!(f.0, 42);

    assert_eq!(g.0[0], 5);
    assert_eq!(h.0[0], 6);
}

#[Singleton(name = "ref")]
fn Run2(
    #[di(ref)] one: &i8,
    #[di(ref, name = "2")] two: &i8,

    #[di(ref)] three: &Option<i16>,
    #[di(ref, option)] four: Option<&i16>,

    #[di(ref, default = &0)] zero: &i32,
    #[di(ref, default = &42)] forty_two: &i32,

    #[di(ref)] five: &Vec<i64>,
    #[di(ref, vec)] six: Vec<&i64>,
) {
    assert_eq!(one, &1);
    assert_eq!(two, &2);

    assert_eq!(three, &Some(3));
    assert_eq!(four, Some(&4));

    assert_eq!(zero, &0);
    assert_eq!(forty_two, &42);

    assert_eq!(five, &vec![5]);
    assert_eq!(six, vec![&6]);
}

mod alias {
    use rudi::Singleton;

    type OneAndTwo<'a> = &'a i8;

    type Three<'a> = &'a Option<i16>;
    type Four<'a> = Option<&'a i16>;

    type ZeroAndFortyTwo<'a> = &'a i32;

    type Five<'a> = &'a Vec<i64>;
    type Six<'a> = Vec<&'a i64>;

    #[Singleton(name = "ref alias")]
    fn Run3(
        #[di(ref = i8)] one: OneAndTwo<'_>,
        #[di(ref = i8, name = "2")] two: OneAndTwo<'_>,

        #[di(ref = Option<i16>)] three: Three<'_>,
        #[di(ref = i16, option)] four: Four<'_>,

        #[di(ref = i32, default = &0)] zero: ZeroAndFortyTwo<'_>,
        #[di(ref = i32, default = &42)] forty_two: ZeroAndFortyTwo<'_>,

        #[di(ref = Vec<i64>)] five: Five<'_>,
        #[di(ref = i64, vec)] six: Six<'_>,
    ) {
        assert_eq!(one, &1);
        assert_eq!(two, &2);

        assert_eq!(three, &Some(3));
        assert_eq!(four, Some(&4));

        assert_eq!(zero, &0);
        assert_eq!(forty_two, &42);

        assert_eq!(five, &vec![5]);
        assert_eq!(six, vec![&6]);
    }
}

fn main() {
    let mut cx = Context::auto_register();
    cx.resolve::<()>();
    cx.resolve_with_name::<()>("ref");
    cx.resolve_with_name::<()>("ref alias");
}
```
