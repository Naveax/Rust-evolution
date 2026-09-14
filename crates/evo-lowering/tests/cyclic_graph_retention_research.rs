use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

static STRONG_DROPS: AtomicUsize = AtomicUsize::new(0);
static WEAK_DROPS: AtomicUsize = AtomicUsize::new(0);

struct StrongNode {
    next: RefCell<Option<Rc<StrongNode>>>,
}

impl Drop for StrongNode {
    fn drop(&mut self) {
        STRONG_DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

struct WeakNode {
    parent: RefCell<Weak<WeakNode>>,
}

impl Drop for WeakNode {
    fn drop(&mut self) {
        WEAK_DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn strong_rc_cycle_retains_nodes_after_external_owners_drop() {
    STRONG_DROPS.store(0, Ordering::SeqCst);
    {
        let left = Rc::new(StrongNode {
            next: RefCell::new(None),
        });
        let right = Rc::new(StrongNode {
            next: RefCell::new(None),
        });
        *left.next.borrow_mut() = Some(Rc::clone(&right));
        *right.next.borrow_mut() = Some(Rc::clone(&left));
        assert_eq!(Rc::strong_count(&left), 2);
        assert_eq!(Rc::strong_count(&right), 2);
    }
    assert_eq!(
        STRONG_DROPS.load(Ordering::SeqCst),
        0,
        "a strong Rc cycle retains both nodes rather than collecting them"
    );
}

#[test]
fn weak_back_edge_allows_nodes_to_drop_without_hidden_collection() {
    WEAK_DROPS.store(0, Ordering::SeqCst);
    {
        let parent = Rc::new(WeakNode {
            parent: RefCell::new(Weak::new()),
        });
        let child = Rc::new(WeakNode {
            parent: RefCell::new(Weak::new()),
        });
        *child.parent.borrow_mut() = Rc::downgrade(&parent);
        assert_eq!(Rc::strong_count(&parent), 1);
        assert_eq!(Rc::weak_count(&parent), 1);
        assert!(child.parent.borrow().upgrade().is_some());
    }
    assert_eq!(
        WEAK_DROPS.load(Ordering::SeqCst),
        2,
        "weak back edges must not keep either node alive"
    );
}
