title: Squeezing CPUs for speed
class: animation-fade
layout: true

<!-- This slide will serve as the base layout for all your slides -->
.bottom-bar[
  {{title}}
]

---

class: impact

# {{title}}
## Optimization case study with matrix multiplication

---

# Goals

* Explore modern CPU capabilities for faster execution
  - Memory hierarchy & caches
  - Parallelism (CPUs, cores, hyper-threading)
  - Vector instructions
* Walk through some usual steps to obtain faster programs
* Demonstrate some tools that help with all that
  - Will be done in Rust
  - Some other languages allow optimizing too, but I like Rust
  - Some languages make optimizing *hard*
* Show a lot of graphs from unscientific benchmarks 😇
  - Numbers for AMD FX-8370 Eight-Core Processor @ 4.2GHz
  - Unless stated otherwise

---

# Case study

* Two huge matrices, compute a product
* Composed of floats
* Simplification for educational purposes
  - Square size
  - Power of two side
* Comparison with the [`armadillo`](http://arma.sourceforge.net) library
  - 8192: 350s (almost 6 minutes)
  - 16384: 2783s (46 minutes)
  - Spoiler: we're going to do better 😈
* Another spoiler: We'll reach 1000× speedup

TODO: Graph for armadillo

---

# Recap: matrix multiplication

* \\(C = A \times B\\)
* \\(C\_{x, y}\\) = \\(\sum\_{i=1}^n A\_{i, y}\cdot B\_{x, i} \\)
* \\(O(n^3)\\)

TODO: Image at the right hand side

---

# Trivial implementation

```rust
for x in 0..w {
    for y in 0..h {
        for p in 0..l {
            into[(x, y)] += a[(p, y)] * b[(x, p)];
        }
    }
}
```

--

* 1024: 135s
* 2048: 1664s
* That's just terrible
* 😭

---

# Step 0: Try to avoid the problem

* Find a library or ready-made solution
* Buy some better HW
* Switch projects
* Pretend it's normal
* Sell something else to the customer
  - Sum the matrices instead of multiplying
  - They won't notice, will they?
* Promise better speed for next version
  - They should have motivation to buy it
* Become a shepherd
  - 🐑 🐑 🐐 🐑

---

# Step 1: Let the compiler do it's job

- And remember to turn **the optimizations** on
- `cargo run --release`
--

* 1024: 15s
* 2048: 323s
* 4096: 2930s

- That's better, but not enough
- About 5× speedup (and no actual work done)
