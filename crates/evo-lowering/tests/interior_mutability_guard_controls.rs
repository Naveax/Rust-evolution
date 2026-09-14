use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn exclusive_owner_can_mutate_refcell_payload_without_dynamic_borrow() {
    let mut cell = RefCell::new(7_i64);
    *cell.get_mut() += 1;
    assert_eq!(cell.into_inner(), 8);
}

#[test]
fn dynamic_borrow_state_ends_with_guard_scope() {
    let cell = RefCell::new(7_i64);
    {
        let shared = cell.borrow();
        assert_eq!(*shared, 7);
        assert!(cell.try_borrow_mut().is_err());
    }
    {
        let mut exclusive = cell.borrow_mut();
        *exclusive += 1;
    }
    assert_eq!(*cell.borrow(), 8);
}

#[test]
fn rc_owner_count_and_refcell_borrow_state_are_independent() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    assert_eq!(Rc::strong_count(&owner), 2);

    let shared = owner.borrow();
    assert_eq!(Rc::strong_count(&owner), 2);
    assert!(alias.try_borrow_mut().is_err());
    drop(shared);

    *alias.borrow_mut() += 1;
    assert_eq!(*owner.borrow(), 8);
    assert_eq!(Rc::strong_count(&owner), 2);
}
