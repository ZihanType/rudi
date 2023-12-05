use rudi::{Context, SingleOwner, Singleton, Transient};

#[Transient(name = "one")]
fn One() -> i32 {
    1
}

#[Singleton(name = "two")]
fn Two() -> i32 {
    2
}

#[SingleOwner(name = "three")]
fn Three() -> i32 {
    3
}

#[test]
fn transient_owned() {
    let mut cx = Context::auto_register();
    assert_eq!(cx.resolve_with_name::<i32>("one"), 1);
}

#[test]
#[should_panic]
fn transient_ref() {
    let mut cx = Context::auto_register();
    assert!(cx.try_just_create_single_with_name::<i32>("one"));
    assert_eq!(cx.get_single_with_name::<i32>("one"), &1);
}

#[test]
fn singleton_owned() {
    let mut cx = Context::auto_register();
    assert_eq!(cx.resolve_with_name::<i32>("two"), 2);
}

#[test]
fn singleton_ref() {
    let mut cx = Context::auto_register();
    assert!(cx.try_just_create_single_with_name::<i32>("two"));
    assert_eq!(cx.get_single_with_name::<i32>("two"), &2);
}

#[test]
#[should_panic]
fn single_owner_owned() {
    let mut cx = Context::auto_register();
    assert_eq!(cx.resolve_with_name::<i32>("three"), 3);
}

#[test]
fn single_owner_ref() {
    let mut cx = Context::auto_register();
    assert!(cx.try_just_create_single_with_name::<i32>("three"));
    assert_eq!(cx.get_single_with_name::<i32>("three"), &3);
}
