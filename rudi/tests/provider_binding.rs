mod components;

use std::{any::TypeId, rc::Rc};

use rudi::{DefaultProvider, Definition, DynProvider};

use crate::components::{Component1, Trait1, Trait2};

#[test]
fn binding_definitions() {
    let provider = Component1::provider();
    test_binding_definitions(provider.binding_definitions());
    test_binding_definitions(DynProvider::from(provider).binding_definitions());
}

fn test_binding_definitions(definitions: Option<&Vec<Definition>>) {
    assert!(definitions.is_some());

    let definitions = definitions.unwrap();
    assert_eq!(definitions.len(), 2);

    let trait1 = &definitions[0];
    assert_eq!(trait1.key.ty.id, TypeId::of::<Rc<dyn Trait1>>());
    assert!(trait1.origin.is_some());
    assert_eq!(
        trait1.origin.as_ref().unwrap().id,
        TypeId::of::<Component1>()
    );

    let trait2 = &definitions[1];
    assert_eq!(trait2.key.ty.id, TypeId::of::<Rc<dyn Trait2>>());
    assert!(trait2.origin.is_some());
    assert_eq!(
        trait2.origin.as_ref().unwrap().id,
        TypeId::of::<Component1>()
    );
}
