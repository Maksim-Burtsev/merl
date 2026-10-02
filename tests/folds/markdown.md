---
title: front matter is not a heading
---
Text before the first heading folds as a section. <!-- f: 4-5 -->
Second line of it. <!-- f: 4-5 -->

# Title <!-- f: 7-44 -->

## Lists <!-- f: 9-26 -->

- one <!-- f: 11-19 -->
- two, wrapped <!-- f: 11-19 -->
  over two lines <!-- f: 11-19 -->
- three, holding a list <!-- f: 14-16 -->
  1. a <!-- f: 14-16 -->
  2. b <!-- f: 14-16 -->

- four, after a blank line <!-- f: 11-19 -->
A lazy line of item four. <!-- f: 11-19 -->

A paragraph ends the list. <!-- f: 9-26 -->

	* a tab-indented list <!-- f: 23-24 -->
	* that tabs make code <!-- f: 23-24 -->

### A deeper heading inside Lists <!-- f: 9-26 -->

## Code <!-- f: 28-44 -->

```rust <!-- f: 30-34 -->
fn main() { <!-- f: 30-34 -->
    // # not a heading <!-- f: 30-34 -->
} <!-- f: 30-34 -->
```

~~~~ <!-- f: 36-38 -->
``` inside a tilde fence <!-- f: 36-38 -->
~~~~

    indented code <!-- f: 40-41 -->
    over two lines <!-- f: 40-41 -->

Title line under a setext rule <!-- f: 28-44 -->
=== <!-- f: 28-44 -->
