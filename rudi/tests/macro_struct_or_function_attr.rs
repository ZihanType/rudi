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
    !cx.contains_singleton_with_name::<i32>("5")
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

#[tokio::test]
async fn struct_or_function_attr() {
    let mut cx = Context::auto_register();

    assert_eq!(cx.resolve::<i8>(), 1);
    assert_eq!(cx.resolve_with_name::<i8>("2"), 2);

    assert!(!cx.contains_singleton_with_name::<i16>("3"));
    assert!(cx.contains_singleton_with_name::<i16>("4"));

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
