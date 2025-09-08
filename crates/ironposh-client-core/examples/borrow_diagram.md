# Borrow Checker Issue Diagram

## The Lifetime and Borrow Relationships

```
LOOP ITERATION 1:
================

Stack Frame                    Heap
-----------                    ----

┌─────────────────┐
│ context         │───────────▶ ┌──────────────┐
│ (owns data)     │             │ Vec<u8>      │
└─────────────────┘             │ [1, 2, 3]    │
         ▲                      └──────────────┘
         │ &'a mut                      ▲
         │                              │
┌─────────────────┐                     │ &'a mut
│ holder          │                     │
│ Option<Builder> │───────▶ ┌──────────────────┐
└─────────────────┘         │ Builder<'a>      │
         ▲                  │ ┌──────────────┐ │
         │ &'b mut          │ │ _data: ──────┼─┘
         │                  │ └──────────────┘ │
         │                  └──────────────────┘
         │                           ▲
         │                           │ &'a mut
┌─────────────────┐                  │
│ _gen            │         ┌──────────────────┐
│ Generator<'a>   │────────▶│ Generator<'a>    │
└─────────────────┘         │ ┌──────────────┐ │
                            │ │ _builder_ref:─┼─┘
                            │ └──────────────┘ │
                            └──────────────────┘

Lifetimes:
- 'a: tied to &mut context borrow
- 'b: tied to &mut holder borrow  
- constraint: 'b: 'a (holder outlives context borrow)

PROBLEM: _gen has type Generator<'a>, so Rust thinks:
- The 'a lifetime (from &mut context) is still "in use"
- Even though we don't access _gen, its mere existence keeps the borrow alive


LOOP ITERATION 2 (ATTEMPTING):
===============================

┌─────────────────┐
│ context         │ ← ❌ Can't borrow again! Previous 'a still "active"
└─────────────────┘    

┌─────────────────┐
│ holder          │ ← ❌ Can't borrow again! Previous 'b still "active"
└─────────────────┘    

The compiler sees:
- _gen from iteration 1 has lifetime 'a (tied to context borrow)
- _gen is still in scope (even if unused)
- Therefore, can't create new borrows of context or holder
```

## Why the Lifetime Persists

```rust
// Simplified view of what the compiler sees:

loop {
    // Iteration 1
    let _gen: Generator<'a> = {
        // 'a begins here (borrowing context)
        let builder = Builder { _data: &mut context.data };
        *holder = Some(builder);
        Generator { _builder_ref: holder.as_mut().unwrap() }
    };
    // 'a is STILL ACTIVE here because _gen: Generator<'a> exists
    
    // Iteration 2
    let _gen2 = create_generator(&mut context, &mut holder);
    //                           ^^^^^^^^^^^^  ^^^^^^^^^^^ 
    //                           ERROR: already borrowed in iteration 1!
}
```

## The Working Pattern (Immediate Consumption)

```
┌─────────────────┐
│ context         │───────▶ data
└─────────────────┘
         ▲
         │ &mut (temporary)
         │
    ┌────▼──────┐
    │ builder   │ (created)
    └────┬──────┘
         │
    ┌────▼──────┐
    │ generator │ (created)
    └────┬──────┘
         │
    ┌────▼──────────────┐
    │ resolve_to_result │ (CONSUMES generator)
    └────┬──────────────┘
         │
    ┌────▼──────┐
    │   Result  │ (owned data, no borrows!)
    └───────────┘
    
All borrows released! ✓
Next iteration can borrow freely
```

## The Problem Pattern (Suspended Generator)

```
┌─────────────────┐
│ context         │───────▶ data
└─────────────────┘
         ▲
         │ &mut (PERSISTENT via 'a)
         │
    ┌────▼──────┐
    │ builder   │ (stored in holder)
    └────┬──────┘
         │
    ┌────▼──────┐
    │ generator │ (returned, NOT consumed)
    └───────────┘
         │
         ▼
    "Lifetime 'a persists..."
    "Can't borrow context again!"
```

## Summary

The issue is that:
1. **Generator<'a>** type carries the lifetime 'a
2. Lifetime 'a is tied to the **&mut context** borrow
3. Even if unused, the generator's existence keeps 'a "alive"
4. Rust won't allow new mutable borrows while 'a is active
5. This prevents the next loop iteration from borrowing context

The working example avoids this by immediately consuming the generator,
which releases all borrows before the next iteration.