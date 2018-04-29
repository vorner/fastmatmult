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

.left-column[
* \\(C = A \times B\\)
* \\(C\_{x, y}\\) = \\(\sum\_{i=1}^n A\_{i, y}\cdot B\_{x, i} \\)
* \\(O(n^3)\\)
]

.right-column[![Multiplication](mult.svg)]

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

---

# Step 2: Find the slow part

* Optimizing takes effort
  - We want to optimize where it makes sense
* Guessing is often wrong
* Let's use a profiler
  - `perf` is usually a good choice
  - `FlameGraph` is a nice extension

```
99.89%    99.58%  measure  measure
        |
        ---fastmatmult::simple::multiply_add
```

---

# Step 2: Find the slow part

.center[<img src="fg.svg" height="95%" width="95%">]

---

# Step 3: Find why it is slow

* Doing too much overall
* IO
* Syscalls
* Thread synchronization
* Branch mispredictions
* Waiting for memory

---

# Step 3: Find why it is slow

TODO: Measure again on hydra

* Common sense and `htop` rules out IO, syscalls and threads
* `perf stat` gives more info (1024×1024)
  - 80G cycles vs. 15G instructions ‒ 5.3 cycle per instruction
  - 2.6G cache accesses, 2.5G misses ‒ 96% misses
  - 2.2G branches, 4.5M misses
???

* Maybe a good time to describe how perf works
* Perf stat and summing up
* Counter for events, when it overflows, a sample is taken
--

* Mostly cache misses are to blame
* Number of instructions too

---

# Memory hierarchy

TODO: Schema of memory hierarchy

???

* Further away from CPU is more mem, but slower
* From some point shared
* Cache lines, evictions
* Preloading

---

# Memory hierarchy: NUMA

TODO: Schema of memory hierarchy with NUMA

???

Note that it can be even worse...

---

# Step 4: Do some research

* Is there a better algorithm?
* Could I precompute or reuse something?

--

- Z-order layout
  * Good for caches
- Strassen algorithm

???

* Let's start with the layout, it looks simpler
* Postpone strassen algorithm for later on, it looks complex

---

# Z-Order

* Split matrix in quarters
  - Can be taken as matrix 2×2
  - Matrix multiplication works on the quarters
* Each quarter is continuous in memory
* Each quarter is encoded recursively
* At certain level, the whole matrix fits into cache

TODO: Image of the Z-Order
TODO: Image of the multiplication
TODO: Image of the recursion and how it fits into cache
TODO: Some code

---

# Problems with recursion

* 2048: 178s
* 4096: 1423s

- There's a cost to recursion
  * It dominates on small tasks
  * Small tasks already fit into cache
- Let's do a hybrid approach
  * Switch to simple from some size

|    size  |    4    |   8    |    16    |    32    |
|----------|--------:|-------:|---------:|---------:|
| **2048** |   35s   |   *33s*|   34s    |    37s   |
| **4096** |   272s  |  *266s*|   283s   |   299s   |

???

* General pattern of hybrid approach
* Where one algorithm is faster on large inputs and another on small ones

---

# Parallelism

* Computing of the quarters is independent
* We can distribute the work between cores
* There's a synchronization cost
  - Makes no sense to share tiny tasks
* Can't expect N× speedup
  - Shared memory bandwidth, caches
  - Shared parts (FPU, scheduler)
  - Cooling and power units

---

# Thread pools (Rayon)

TODO: Code

---

# Results

* Distributing down to the small matrices
* Not distributing smaller than 256

|    size   |    2     |    4     |    8    |   16    |   32   |
|-----------|---------:|---------:|--------:|--------:|-------:|
| **2048s** |    16s   |    6s    |    5s   |   5s    |   5s   |
| **2048c** |     9s   |    5s    |    5s   |   5s    |   5s   |
| **4096s** |   152s   |   48s    |   40s   | *38s*   |  41s   |
| **4096c** |    68s   |   41s    |   39s   |  39s    |  42s   |

---

# SIMD

* Usually, one instruction ‒ one result
* SIMD ‒ a vector in each register
* For example 16×float
* Needs aligned data or load them into the registers
* Fast for long runs
* Stronger hints for cache pre-loading
* Matrices ‒ problems with column access
  - There's acceleration for that, but still slow
* Let's use a library (`faster`)
  - For portability and ease of use
  - Needs nightly Rust now, going to stabilize soon

---

# SIMD

```rust
let columns = b.content.simd_iter(f32s(0.));
let columns = columns.stride(b.width, &pads);
let mut column_data = iter::repeat(0.0).take(b.height).collect();
for (x, mut column) in columns.into_iter().enumerate() {
*   column.scalar_fill(&mut column_data);
    for y in 0..h {
        let row = &a.content[y * l .. (y + 1) * l];
        into[(x, y)] += (row.simd_iter(f32s(0.)),
                column_data.simd_iter(f32s(0.)))
            .zip()
            .simd_reduce(f32s(0.0), |acc, (a, b)| acc + a * b)
            .sum();
    }
}
```

???

* Describe the reason for that highlighted line
* Made it actually much faster

---

# Speeds

* Can be combined with other solutions
  - Goes a bit against the recursive/cache optimisation
  - Recursive, parallelized, with 256 sized fragments
* The column-copy trick alone helps only a little

|   size    |   simple    |   column   |   simd   |     combined   |
|-----------|------------:|-----------:|---------:|---------------:|
| **2048**  |   323s      |    35s     |    6s    |      0.7s      |
| **4096**  |  2930s      |   277s     |   44s    |       30s      |

---

# Strassen

* Similar to the recursive
* Reduces the number of smaller multiplications to 7
* At the cost of some additions and removals
  - Isn't worth for small inputs
  - Wins on large ones
* Exact formulae at [wikipedia](https://en.wikipedia.org/wiki/Strassen_algorithm) or elsewhere

- 2048: 0.6s
- 4096: 4s
- 8192: 27s
- 16384: 182s

---

# TODO

* Links to the code
* Graphs
