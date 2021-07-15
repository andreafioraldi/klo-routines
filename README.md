# klo-routines

Rust cheap coroutines with libc::*context

## Example

```rust
use klo_routines::{flush, KloRoutine};

fn main() {
    let mut cnt = 0;
    let mut func = || {
        for _ in 0..16 {
            flush(cnt);
            cnt += 1;
        }
    };

    let mut klo = KloRoutine::new(&mut func);
    while let Some(n) = klo.resume() {
        println!("{}", n);
    }
    
    // or you can use it as iterator
    // for n in &mut klo {
    //     println!("{}", n);
    // }
}
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

