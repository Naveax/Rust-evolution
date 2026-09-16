use std::cell::Cell;
use std::marker::PhantomData;

struct RefHandle<T> {
    arena: u64,
    index: usize,
    generation: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Copy for RefHandle<T> {}

impl<T> Clone for RefHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

struct RefSlot<T> {
    generation: u64,
    value: Option<T>,
    retired: bool,
}

struct RefArena<T> {
    id: u64,
    slots: Vec<RefSlot<T>>,
    free: Vec<usize>,
}

std::thread_local! {
    static NEXT_ARENA_ID: Cell<u64> = const { Cell::new(1) };
}

fn next_arena_id() -> u64 {
    NEXT_ARENA_ID.with(|next| {
        let id = next.get();
        if id == 0 {
            panic!("arena identity exhausted");
        }
        next.set(id.checked_add(1).unwrap_or(0));
        id
    })
}

fn arena_new<T>() -> RefArena<T> {
    RefArena {
        id: next_arena_id(),
        slots: Vec::new(),
        free: Vec::new(),
    }
}

fn arena_insert<T>(arena: &mut RefArena<T>, value: T) -> RefHandle<T> {
    if let Some(index) = arena.free.pop() {
        let slot = &mut arena.slots[index];
        debug_assert!(!slot.retired && slot.value.is_none());
        slot.value = Some(value);
        return RefHandle {
            arena: arena.id,
            index,
            generation: slot.generation,
            _marker: PhantomData,
        };
    }

    let index = arena.slots.len();
    arena.slots.push(RefSlot {
        generation: 0,
        value: Some(value),
        retired: false,
    });
    RefHandle {
        arena: arena.id,
        index,
        generation: 0,
        _marker: PhantomData,
    }
}

fn arena_get<T>(arena: &RefArena<T>, handle: RefHandle<T>) -> Option<&T> {
    if handle.arena != arena.id {
        return None;
    }
    arena
        .slots
        .get(handle.index)
        .filter(|slot| !slot.retired && slot.generation == handle.generation)
        .and_then(|slot| slot.value.as_ref())
}

fn arena_remove<T>(arena: &mut RefArena<T>, handle: RefHandle<T>) -> Option<T> {
    if handle.arena != arena.id {
        return None;
    }
    let slot = arena.slots.get_mut(handle.index)?;
    if slot.retired || slot.generation != handle.generation {
        return None;
    }
    let value = slot.value.take()?;
    if slot.generation == u64::MAX {
        slot.retired = true;
    } else {
        slot.generation += 1;
        arena.free.push(handle.index);
    }
    Some(value)
}

fn input_int() -> i64 {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("failed to read integer input");
    input
        .trim()
        .parse::<i64>()
        .expect("expected signed integer input")
}

fn main() {
    let n = input_int();
    let mut items = arena_new::<i64>();
    let mut sum = 0_i64;
    let mut i = 0_i64;

    for _ in 0..n {
        let stale = arena_insert(&mut items, i);
        if let Some(&value) = arena_get(&items, stale) {
            sum += value;
        } else {
            sum += 1_000_000_000;
        }

        if let Some(removed) = arena_remove(&mut items, stale) {
            sum += removed;
        } else {
            sum += 1_000_000_000;
        }

        let fresh = arena_insert(&mut items, i + 1);
        if arena_get(&items, stale).is_some() {
            sum += 1_000_000_000;
        } else {
            sum += 1;
        }

        if let Some(&value2) = arena_get(&items, fresh) {
            sum += value2;
        } else {
            sum += 1_000_000_000;
        }

        if let Some(removed2) = arena_remove(&mut items, fresh) {
            sum += removed2;
        } else {
            sum += 1_000_000_000;
        }

        i += 1;
    }

    println!("{sum}");
}
