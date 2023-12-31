# Rudi

[![Crates.io version](https://img.shields.io/crates/v/rudi.svg?style=flat-square)](https://crates.io/crates/rudi)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/rudi)

[English](./README.md) | 简体中文

Rudi - 一个开箱即用的 Rust 依赖注入框架。

```rust
use rudi::{Context, Singleton, Transient};

// 将 `fn(cx) -> A { A }` 注册为 `A` 的构造函数
#[derive(Debug)]
#[Transient]
struct A;

#[derive(Debug)]
struct B(A);

// 将 `fn(cx) -> B { B::new(cx.resolve::<A>()) }` 注册为 `B` 的构造函数
#[Transient]
impl B {
    #[di]
    fn new(a: A) -> B {
        B(a)
    }
}

// 将 `fn(cx) -> C { C::B(cx.resolve::<B>()) }` 注册为 `C` 的构造函数
#[allow(dead_code)]
#[Transient]
enum C {
    A(A),

    #[di]
    B(B),
}

// 将 `fn(cx) -> () { Run(cx.resolve::<B>(), cx.resolve::<C>()) }` 注册为 `()` 的构造函数
#[Singleton]
fn Run(b: B, c: C) {
    println!("{:?}", b);
    assert!(matches!(c, C::B(_)));
}

fn main() {
    // 自动注册所有标记了 `#[Singleton]`、`#[Transient]` 或 `#[SingleOwner]` 属性宏的类型和函数
    let mut cx = Context::auto_register();

    // 从 `Context` 中获取一个 `()` 的实例，这将会调用 `Run` 函数
    // 这等价于 `cx.resolve::<()>();`
    cx.resolve()
}
```

## 特性

- 3 种作用域: [`Singleton`](https://docs.rs/rudi/latest/rudi/enum.Scope.html#variant.Singleton), [`Transient`](https://docs.rs/rudi/latest/rudi/enum.Scope.html#variant.Transient) and [`SingleOwner`](https://docs.rs/rudi/latest/rudi/enum.Scope.html#variant.SingleOwner) ([example](./examples/all-scope/))。
- 异步函数和异步构造器。
- 可以用在 `struct`、`enum`、`impl block` 和 `function` 上的属性宏。
- 手动注册和自动注册 (感谢 [inventory](https://github.com/dtolnay/inventory))。
- 方便的绑定 trait 实现和 trait 对象。
- 使用类型和名称区分不同的实例。
- 泛型 (但是必须单态化后手动注册) ([example](./examples/hello-world-with-generic/))。
- 条件注册 ([example](./examples/condition/))。
- 引用 (只能是 `Singleton` 和 `SingleOwner` 作用域) ([example](./examples/reference/))。

## 一个更复杂的例子

```rust
use std::{fmt::Debug, rc::Rc};

use rudi::{Context, Singleton, Transient};

// 将 `async fn(cx) -> i32 { 42 }` 注册为 `i32` 的构造函数，并将该 `i32` 类型的实例命名为 `"number"`
#[Singleton(name = "number")]
async fn Number() -> i32 {
    42
}

// 注册 `async fn(cx) -> Foo { Foo { number: cx.resolve_with_name_async("number").await } }` 为 `Foo` 的构造函数，
// 并将该 `Foo` 类型的实例命名为 `"foo"`
#[derive(Debug, Clone)]
#[Singleton(async, name = "foo")]
struct Foo {
    #[di(name = "number")]
    number: i32,
}

#[derive(Debug)]
struct Bar(Foo);

impl Bar {
    fn into_debug(self) -> Rc<dyn Debug> {
        Rc::new(self)
    }
}

// 将 `async fn(cx) -> Bar { Bar::new(cx.resolve_with_name_async("foo").await).await }` 注册为 `Bar` 的构造函数，
//
// 将 `Debug` trait 的实现和 `Debug` trait 对象绑定，
// 这将会注册 `async fn(cx) -> Rc<dyn Debug> { Bar::into_debug(cx.resolve_async().await) }` 为 `Rc<dyn Debug>` 的构造函数。
#[Transient(binds = [Self::into_debug])]
impl Bar {
    #[di]
    async fn new(#[di(name = "foo")] f: Foo) -> Bar {
        Bar(f)
    }
}

#[Singleton]
async fn Run(bar: Bar, debug: Rc<dyn Debug>, #[di(name = "foo")] f: Foo) {
    println!("{:?}", bar);
    assert_eq!(format!("{:?}", bar), format!("{:?}", debug));
    assert_eq!(format!("{:?}", bar.0.number), format!("{:?}", f.number));
}

#[tokio::main]
async fn main() {
    let mut cx = Context::auto_register();

    cx.resolve_async().await
}
```

更多例子可以在 [examples](./examples/) 和 [tests](./rudi/tests/) 目录中找到。

## 鸣谢

- [Koin](https://github.com/InsertKoinIO/koin): 本项目的 API 设计和测试用例受到了 Koin 的启发。
- [inventory](https://github.com/dtolnay/inventory): 本项目使用 inventory 实现了自动注册，使得 Rust 的自动注册变得非常简单。

## 做出贡献

感谢您的帮助改进项目！我们很高兴有您的加入！

## 开源许可

本项目使用 Apache-2.0 和 MIT 双重许可，您可以在以下两个许可之一下自由使用本项目的代码：

- Apache License, Version 2.0,([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

### 贡献

除非您另有明确声明，否则您有意提交以纳入作品的任何贡献（如 Apache-2.0 许可中的定义）均应获得上述双重许可，且无任何附加条款或条件。
