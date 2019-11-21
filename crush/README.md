<h1 align="center">Crush (Compressed Right Hand Side in Rust)</h1>

<p align="center">
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/AUTHORS"><img src="https://img.shields.io/badge/authors-SimulaUIB-orange.svg"></a>
    <a href="https://github.com/Simula-UiB/CryptaPath/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>


__Crush__ is a library made for solving multiple right hand side system of equations over GF(2) using Bdds.

This library was developped following the work done on the initial paper *"[Solving Compressed Right Hand Side Equation Systems with Linear Absorption][crhs]"* and introduce new methods of solving CRHS equation systems by using "dropping" of variables.

## License

Crush is licensed under the MIT License.

* MIT license ([LICENSE](../LICENSE) or http://opensource.org/licenses/MIT)


**WARNING:** This library was developed in an academic context and no part of this code should be use in any production system.

## Overview

This library implements a way of solving system of equations over GF(2) with multiple right hand side using Bdds. For this we provide
3 rust modules that can be used together : 

- [`algebra`](src/algebra): Rust module that provides operations on matrices over GF(2)
- [`soc`](src/soc): Rust module that provides a memory representation of a system of Bdds and the apis to mutate it safely with the available operations
- [`solver`](src/solver): Rust module that provides a way of defining Solvers. structures holding the strategy which will use the [`soc`](soc) apis to absorb all linear dependencies inside a system of Bdds, making every remaining path a valid solution

## Build guide

We target the stable channel of Rust.

To build you have first to install rust (you can follow the guide from the [`official website`](https://www.rust-lang.org/tools/install).

You can then run 
```bash
git clone https://github.com/Simula-UiB/CryptaPath.git
cd CryptaPath/Crush
cargo build --release
```

You can run the unit test for the modules [`algebra`](src/algebra) and [`soc`](src/soc) using :

```bash
cargo test
``` 

Finally to make the documentation for this library you can use

```bash
cargo doc --no-deps
```

The documentation will be available in [`target/release/doc/crush/all.html`], which you can open in your browser.
If you want the documentation for [`Node`](src/soc/node.rs) and [`Level`](src/soc/level.rs) you may add the flag `--document-private-items`.

## .bdd file format

One of the way to load a system of Bdd is to use a .bdd file and the function `parse_system_spec_from_file` from the [`utils`](soc/utils.rs) module.

The specification for the file is as follows :

```text
nr of unique vars
nr of bdds in the system
bdd_id number_of_levels_in_this_bdd
lhs (a linear combination of variables id, ex: 13+3+35) : rhs (nodes and links, format: (node_id;id_to_0edge,id_to_1edge) )|
...
last_level (not left hand side, one node with both edges pointing to nothing)
---
(next bdd)
---
(next bdd)
---
...
(last bdd)
---
```

Things to note:
- ":" is the divider between the lhs and rhs
- "|" is the end of level marker
- "---" is the end of bdd marker
- "id_to_0edge"/"id_to_1edge" is the node_id which the 0/1 edge points to with a node_id of 0 means that this edge points to nothing

## Example of a whole solving

You can find an example of a complete solving process (including fixing and printing the solutions) in the tool [`CryptaPath`][CryptaPath].

**Warning:** This implementation is monothreaded but can be very heavy on RAM consumption (on big systems it can easily grow to 200 GB of RAM and more). Be mindful of this if you are running this on cloud engines or constraint servers.

If something was not covered in this README please check the documentation.

[crhs]: https://link.springer.com/chapter/10.1007%2F978-3-642-30615-0_27
[CryptaPath]: https://github.com/Simula-UiB/CryptaPath