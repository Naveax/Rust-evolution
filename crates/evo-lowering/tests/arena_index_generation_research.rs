#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handle {
    index: usize,
    generation: u64,
}

#[derive(Debug)]
struct Slot<T> {
    generation: u64,
    value: Option<T>,
}

fn resolve<T>(slots: &[Slot<T>], handle: Handle) -> Option<&T> {
    slots
        .get(handle.index)
        .filter(|slot| slot.generation == handle.generation)
        .and_then(|slot| slot.value.as_ref())
}

#[test]
fn plain_vec_index_can_silently_rebind_after_slot_reuse() {
    let mut nodes = vec![5_i64, 7_i64];
    let stale_index = 1usize;
    assert_eq!(nodes.pop(), Some(7));
    nodes.push(99);
    assert_eq!(
        nodes.get(stale_index),
        Some(&99),
        "plain usize identity is insufficient when removed slots may be reused"
    );
}

#[test]
fn generation_checked_handle_rejects_reused_slot() {
    let mut slots = vec![
        Slot {
            generation: 0,
            value: Some(5_i64),
        },
        Slot {
            generation: 0,
            value: Some(7_i64),
        },
    ];
    let stale = Handle {
        index: 1,
        generation: 0,
    };
    assert_eq!(resolve(&slots, stale), Some(&7));

    slots[1].value = None;
    slots[1].generation += 1;
    slots[1].value = Some(99);

    assert_eq!(resolve(&slots, stale), None);
    let fresh = Handle {
        index: 1,
        generation: 1,
    };
    assert_eq!(resolve(&slots, fresh), Some(&99));
}
