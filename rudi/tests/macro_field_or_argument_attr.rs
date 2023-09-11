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

#[test]
fn field_or_argument_attr() {
    let mut cx = Context::auto_register();
    cx.resolve::<()>();
    cx.resolve_with_name::<()>("ref");
}
