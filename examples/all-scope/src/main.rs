use rudi::{Context, SingleOwner, Singleton, Transient};

#[derive(Clone)]
struct Cloneable(i32);

struct NotCloneable(i32);

#[Transient(name = "transient_one")]
fn One() -> NotCloneable {
    NotCloneable(1)
}

#[Singleton(name = "singleton_two")]
fn Two() -> Cloneable {
    Cloneable(2)
}

#[SingleOwner(name = "single_owner_three")]
fn Three() -> NotCloneable {
    NotCloneable(3)
}

#[Singleton]
fn Run(
    // Transient only get owned
    #[di(name = "transient_one")] owned_one: NotCloneable,
    // Singleton can get owned and ref, but must implement Clone
    #[di(name = "singleton_two")] owned_two: Cloneable,
    #[di(name = "singleton_two", ref)] ref_two: &Cloneable,
    // SingleOwner only get ref
    #[di(name = "single_owner_three", ref)] ref_three: &NotCloneable,
) {
    assert_eq!(owned_one.0, 1);
    assert_eq!(owned_two.0, 2);
    assert_eq!(ref_two.0, 2);
    assert_eq!(ref_three.0, 3);
}

fn main() {
    let mut cx = Context::auto_register();
    cx.resolve()
}
