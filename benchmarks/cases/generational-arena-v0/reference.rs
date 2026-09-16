struct __EvoHandle<T> {
    arena: u64,
    index: usize,
    generation: u64,
    _marker: std::marker::PhantomData<fn() -> T>,
}
impl<T> Copy for __EvoHandle<T> {}
impl<T> Clone for __EvoHandle<T> {
    fn clone(&self) -> Self { *self }
}
struct __EvoSlot<T> {
    generation: u64,
    value: Option<T>,
    retired: bool,
}
struct __EvoArena<T> {
    id: u64,
    slots: Vec<__EvoSlot<T>>,
    free: Vec<usize>,
}
std::thread_local! {
    static __EVO_NEXT_ARENA_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}
fn __evo_next_arena_id() -> u64 {
    __EVO_NEXT_ARENA_ID.with(|next| {
        let id = next.get();
        if id == 0 { panic!("arena identity exhausted"); }
        next.set(id.checked_add(1).unwrap_or(0));
        id
    })
}
fn __evo_arena_new<T>() -> __EvoArena<T> {
    __EvoArena { id: __evo_next_arena_id(), slots: Vec::new(), free: Vec::new() }
}
fn __evo_arena_insert<T>(arena: &mut __EvoArena<T>, value: T) -> __EvoHandle<T> {
    if let Some(index) = arena.free.pop() {
        let slot = &mut arena.slots[index];
        debug_assert!(!slot.retired && slot.value.is_none());
        slot.value = Some(value);
        return __EvoHandle { arena: arena.id, index, generation: slot.generation, _marker: std::marker::PhantomData };
    }
    let index = arena.slots.len();
    arena.slots.push(__EvoSlot { generation: 0, value: Some(value), retired: false });
    __EvoHandle { arena: arena.id, index, generation: 0, _marker: std::marker::PhantomData }
}
fn __evo_arena_get<T>(arena: &__EvoArena<T>, handle: __EvoHandle<T>) -> Option<&T> {
    if handle.arena != arena.id { return None; }
    arena.slots.get(handle.index)
        .filter(|slot| !slot.retired && slot.generation == handle.generation)
        .and_then(|slot| slot.value.as_ref())
}
fn __evo_arena_remove<T>(arena: &mut __EvoArena<T>, handle: __EvoHandle<T>) -> Option<T> {
    if handle.arena != arena.id { return None; }
    let slot = arena.slots.get_mut(handle.index)?;
    if slot.retired || slot.generation != handle.generation { return None; }
    let value = slot.value.take()?;
    if slot.generation == u64::MAX {
        slot.retired = true;
    } else {
        slot.generation += 1;
        arena.free.push(handle.index);
    }
    Some(value)
}

fn __evo_input_int() -> i64 {
    let mut __evo_input = String::new();
    std::io::stdin()
        .read_line(&mut __evo_input)
        .expect("failed to read integer input");
    __evo_input
        .trim()
        .parse::<i64>()
        .expect("expected signed integer input")
}

fn main() {
    let n = __evo_input_int();
    let mut items = __evo_arena_new::<i64>();
    let mut sum = 0_i64;
    let mut i = 0_i64;

    for _ in 0..n {
        let stale = __evo_arena_insert(&mut items, i);
        if let Some(&value) = __evo_arena_get(&items, stale) {
            sum += value;
        } else {
            sum += 1_000_000_000;
        }

        if let Some(removed) = __evo_arena_remove(&mut items, stale) {
            sum += removed;
        } else {
            sum += 1_000_000_000;
        }

        let fresh = __evo_arena_insert(&mut items, i + 1);
        if __evo_arena_get(&items, stale).is_some() {
            sum += 1_000_000_000;
        } else {
            sum += 1;
        }

        if let Some(&value2) = __evo_arena_get(&items, fresh) {
            sum += value2;
        } else {
            sum += 1_000_000_000;
        }

        if let Some(removed2) = __evo_arena_remove(&mut items, fresh) {
            sum += removed2;
        } else {
            sum += 1_000_000_000;
        }

        i += 1;
    }

    println!("{sum}");
}
