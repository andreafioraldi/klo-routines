# klo-routines

Rust cheap coroutines with libc::*context

## Example

```rust
let mut cnt = 0;
let mut func = || {
    for _ in 0..16 {
        yield_(cnt);
        cnt += 1;
    }
};

let mut klo = KloRoutine::new(&mut func);
println!("{}", klo.resume().unwrap()); // 0
println!("{}", klo.resume().unwrap()); // 1
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

