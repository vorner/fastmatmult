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

Michal Vaner

[michal.vaner@avast.com](mailto:michal.vaner@avast.com)

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

.left-column[
* Two matrices, compute a product
* Composed of floats
* Simplification for educational purposes
  - Square size
  - Power of two side
* Comparison with the [`armadillo`](http://arma.sourceforge.net) library
  - 8192: 350s (almost 6 minutes)
  - 16384: 2783s (46 minutes)
  - Spoiler: we're going to do better 😈
* Another spoiler: We'll reach 1000× speedup
]

.right-column[
![Armadillo](arm.svg)
]

---

# Recap: matrix multiplication

.left-column[
* Cell is a dot-product of a row from the left and column from the right
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
* Promise better speed for the next version
  - They should have motivation to buy it
* Become a shepherd
  - 🐑 🐑 🐐 🐑

---

# Step 1: Let the compiler do it's job

- And remember to turn **the optimizations** on
- `cargo run --release`
- Possibly with CPU-specific features
  * `-march=native`
  * `-C target-cpu=native`
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

???

* I'll explain the ones that turn out to be our problems.
* Others on request.

---

# Step 3: Find why it is slow

* Common sense and `htop` rules out IO, syscalls and threads
* `perf stat` gives more info (1024×1024)
  - 60G cycles vs. 30G instructions ‒ 2 cycles per instruction
  - 1.5G cache accesses vs. 1G cache misses
  - 4G branches vs. 1M mispredicted
???

* Point out the low instruction vs. cycle count (hyperscalar processor)
* Maybe a good time to describe how perf works
* Perf stat and summing up
* Counter for events, when it overflows, a sample is taken
--

* Mostly cache misses are to blame
* Number of instructions too

---

# Perf

```
 Performance counter stats for './target/release/measure -s a.out b.out':

    60,535,345,914      cycles
    30,287,912,793      instructions              #    0.50  insn per cycle
     4,340,795,281      branches
         1,160,771      branch-misses             #    0.03% of all branches
     1,433,441,916      cache-references
     1,077,840,136      cache-misses              #   75.192 % of all cache refs

      14.154347044 seconds time elapsed
```

---

# Memory hierarchy

![Memory hierarchy](hier.svg)

* Cache lines, pages, predictors...
* Better to access recently accessed data
* Or data close to recently accessed data
* Linear or other predictable pattern is good

???

* Further away from CPU is more mem, but slower
* From some point shared
* Cache lines, evictions
* Preloading

---

# Memory hierarchy: NUMA

![Numa hierarchy](numa.svg)

???

Note that it can be even worse...

---

# Matrix layout in memory

.center[
![Matrix & cache lines](cache-matrix.svg)
]

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

---

# Z-Order: splitting schema

.left-column[
.center[![Z Order](z-order.svg)]
]

--

.right-column[
.center[![Z order 2](z-order-2.svg)]
]

---

# Z-Order: multiplication

```rust
fn mult(r: &mut [Element], a: &[Element], b: &[Element], size: usize) {
    if size == 1 {
        r[0] += a[0] * b[0];
    } else {
        let s = size / 2;
        let (a11, a12, a21, a22) = quads!(a);
        let (b11, b12, b21, b22) = quads!(b);
        let (r11, r12, r21, r22) = quads!(mut r);

        mult(r11, a11, b11, s);
        mult(r11, a12, b21, s);
        ...
        mult(r22, a22, b22, s);
    }
}
```

---

# Problems with recursion

.left-column[
* 2048: 178s
* 4096: 1423s
* About 2× better

- There's a cost to recursion
  * It dominates on small tasks
  * Small tasks already fit into cache
- Let's do a hybrid approach
  * Switch to simple from some size

|    size  |    4    |   8    |    16    |    32    |
|----------|--------:|-------:|---------:|---------:|
| **2048** |   35s   |   *33s*|   34s    |    37s   |
| **4096** |   272s  |  *266s*|   283s   |   299s   |
]

.right-column[
![Recursion](recursion.svg)

* Another 5× speedup
]

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

```rust
fn run<I: Send, F: Fn(&mut I) + Send + Sync>(
    size: usize,
    tasks: &mut [I], f: F
) {
    if size >= Limit::USIZE {
        tasks
            // Potentially runs on multiple threads
*           .into_par_iter()
            .for_each(f);
    } else {
        for t in tasks {
            f(t);
        }
    }
}
```

---

# Results

* Distributing down to the small matrices (`s`)
* Not distributing smaller than 256 (`c`)

|    size   |    2     |    4     |    8    |   16    |   32   |
|-----------|---------:|---------:|--------:|--------:|-------:|
| **2048s** |    16s   |    6s    |    5s   |   5s    |   5s   |
| **2048c** |     9s   |    5s    |    5s   |   5s    |   5s   |
| **4096s** |   152s   |   48s    |   40s   | *38s*   |  41s   |
| **4096c** |    68s   |   41s    |   39s   |  39s    |  42s   |

* Further 6× speedup on 8-core CPU
* Can expect more with more cores
  - Measured 14× on machine with 2×10 cores with 2×HT (40 virtual cores, 20
    physical)

---

# SIMD (Single Instruction Multiple Data)

.left-column[
* Usually, one instruction ‒ one result
* SIMD ‒ a vector in each register
* For example registers for 16 floats
* Fast on long arrays
* Stronger hints for cache pre-loading?
* Problems with column access
  - Acceleration for that, still slow
* Let's use a library (`faster`)
  - For portability and ease of use
  - Needs nightly Rust now, going to stabilize soon
]

.right-column[
![SIMD](simd.svg)
]

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
*           .simd_reduce(f32s(0.0), |acc, (a, b)| acc + a * b)
            .sum();
    }
}
```

???

* Describe the reason for that first highlighted line
* Made it actually much faster

---

# Speeds

* Can be combined with other solutions
  - Goes a bit against the recursive/cache optimisation
  - Recursive, parallelized, with 256 sized fragments
* The column-copy trick alone helps only a little

|   size    |   simple    |   column   |   simd   | recursive+simd |
|-----------|------------:|-----------:|---------:|---------------:|
| **2048**  |   323s      |    35s     |    6s    |      0.7s      |
| **4096**  |  2930s      |   277s     |   44s    |       6s      |

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
  * 2783s for Armadillo
  * 348s for theoretical parallelized Armadillo

---

# Final profile

```
 Performance counter stats for './target/release/strass a.out b.out':

   115,374,630,652      cycles
   187,871,575,042      instructions              #    1.63  insn per cycle
    12,756,647,765      branches
       163,876,845      branch-misses             #    1.28% of all branches
     3,187,674,343      cache-references
       100,655,288      cache-misses              #    3.158 % of all cache refs

       6.254030767 seconds time elapsed
```

???

* As can be seen, it is not only faster, but we got much better in instructions
  per cycle & cache-misses

---

# Source code

* https://github.com/vorner/fastmatmult
* Somewhat templated to assemble all the measured variants
* Needs specific version of rust nightly
  - SIMD is about to stabilize, therefore a lot of last-minute changes.

---

# Buldozer 8-core

.center[
![Buldozer](buldozer.svg)
]

---

# Xeon 2×10×2HT

.center[
![40-cores](beast.svg)
]

---

# Celeron 4-core

.center[
![Celeron](celeron.svg)
]
