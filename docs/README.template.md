PyMeta - Powerful Python-based metaprogramming for Rust
---

[![GitHub Repo](https://img.shields.io/badge/GitHub-shBLOCK/pymeta-purple?logo=github)](https://github.com/shBLOCK/pymeta)
[![crates.io Version](https://img.shields.io/crates/v/pymeta?logo=rust)](https://crates.io/crates/pymeta)
[![docs.rs](https://img.shields.io/badge/docs.rs-pymeta-blue?logo=docs.rs)](https://docs.rs/pymeta)
[![coverage](https://img.shields.io/codecov/c/github/shBLOCK/pymeta?logo=codecov)](https://codecov.io/gh/shBLOCK/pymeta)

Generate and transform Rust code by **running Python code at compile time**.<br>
**Write Python code alongside normal Rust code** for seamless inline metaprogramming.<br>
Define intuitive **Python-based macros**.<br>
Seamless **integration with tooling and IDEs**.

# Intro Example: Vector Structs
```rust
// Generate vector structs with PyMeta inline metaprogramming.
pymeta! {
    INCLUDE!("BACKEND_TEST/include/vecs_struct.input.rs")
}
```
> **Expanded code from `pymeta!`:**
> ```rust
> INCLUDE!("BACKEND_TEST/include/vecs_struct.output.rs")
> ```
```rust
fn main() {
    // PyMeta keeps source-location information so Rust tooling and IDEs
    // know where generated code came from.
    // For example:
    
    // When compiling this example, you should get warnings like:
    // warning: struct `Vec2` is never constructed
    // --> examples/vecs.rs:12:16
    //    |
    // 12 |         struct Vec~$dims$ {
    //    |                ^^^^^^^^^^

    // In most IDEs, you can "navigate into" (aka. ctrl-click) the `Vec3` below.
    // You should jump to the `Vec~$dims$` part above.
    // Your IDE may also "gray-out" the `Vec~$dims$` part, because some struct generated from it are unused.
    let vec = Vec3 { x: 1.0, y: 2.0, z: 3.0 };
}
```
(See below for the next part of this example)

# Features
- Generate & manipulate any Rust code in Python
- Write metaprogramming Python **alongside** normal Rust code, seamlessly
    - Generate code using any **Python control flow** (`for`, `if`, `match`, `with`, functions, etc.)
- Dev experience & IDE integration:
    - Preserves `Span` (**source location**) information
        - Tooling and IDEs know where each part of the generated code came from
        - Compile-time Python errors give **tracebacks** into Rust source files
- Writing Python-based macros and proc-macros
    - Much more powerful than `macro_rules!`, much less boilerplate than Rust-based proc-macro
    - No need for a separate proc-macro crate
- Reusing code: define "PyMeta modules" in normal Rust modules to reuse metaprogramming code and data

# Getting Started
## Installation
Add `pymeta` as a dependency in your `Cargo.toml` file.<br>
**If you are on nightly Rust, it is highly recommended to enable PyMeta's `nightly_diagnostic` feature
so you get much better diagnostics outputs (error messages).**

PyMeta currently only support the official CPython (**>=3.12**) (through [PyO3](https://pyo3.rs/)).
So a CPython installation is required to compile a crate that uses PyMeta.<br>
There are plans to support embedded Python interpreters (e.g. MicroPython) in the future
to remove the dependency on a CPython environment.<br>
PyO3 will use the current virtualenv or the system's `python`/`python3` executable by default.<br>
You can set the env var `PYO3_PYTHON=<path to Python executable>` to use a custom interpreter.<br>
For more information, see [PyO3's documentation on configuring the Python version](https://pyo3.rs/latest/building-and-distribution.html#configuring-the-python-version).
## Usage
Most features of PyMeta are documented with examples and the code comments in the examples.<br>
Please read through the examples and their comments to learn to use PyMeta.<br>
It is recommended to read through the examples in sequence.
## IDE
PyMeta has been thoroughly tested in the [RustRover](https://www.jetbrains.com/rust/) IDE.
Other IDEs would probably work, but I have not thoroughly tested them with PyMeta.<br>
Currently there's one [RustRover bug](https://youtrack.jetbrains.com/projects/RUST/issues/RUST-20689/Unexpected-merging-of-spanned-identifiers-in-output-of-proc-macro) affecting some advanced features of PyMeta
(details explained in examples below), but hopefully it gets fixed soon.<br>
If your IDE is having trouble with PyMeta macros, feel free to report them to this repo.

<details open>
<summary>

# Intro Example (Cont.): Vector Structs
</summary>

(See above for the first part of this example)

Next, let's implement some binary operation traits for our vector structs.

```rust
pymeta! {
    INCLUDE!("BACKEND_TEST/include/vecs_ops.input.rs")
}
```
> **Expansion:**
> ```rust
> impl std::ops::Add for Vec2 {
>     type Output = Vec2;
>     fn add(self, rhs: Self) -> Self {
>         Self {
>             x: self.x + rhs.x,
>             y: self.y + rhs.y,
>         }
>     }
> }
> impl std::ops::AddAssign for Vec2 {
>     fn add_assign(&mut self, rhs: Self) {
>         self.x += rhs.x;
>         self.y += rhs.y;
>     }
> }
> // ... (200+ more lines, see below for full expansion)
> ```
> <details>
> <summary>Full expansion</summary>
> 
> ```rust
> INCLUDE!("BACKEND_TEST/include/vecs_ops.output.rs")
> ```
> </details>

> [!NOTE]
> The following examples assume that understanding of some Rust proc-macro concepts.<br>
> Most notably, if you are not familiar with the concept of Rust code "Token"s,
> please refer to [Rust's TokenTree documentation](https://doc.rust-lang.org/proc_macro/enum.TokenTree.html) while reading the following examples. 

Then, let's add [swizzle](https://en.wikipedia.org/wiki/Swizzling_(computer_graphics)) operations to the vectors.
Since this involves a LOT of functions to cover all possible arrangements,
we will put them in traits to not pollute the namespace when swizzles are not needed.

```rust
pymeta! {
    INCLUDE!("BACKEND_TEST/include/vecs_swizzle.input.rs")
}
```
> **Expansion:**
> ```rust
> trait Vec2Swizzle {
>     fn xx(self) -> Vec2;
>     fn xy(self) -> Vec2;
>     fn yx(self) -> Vec2;
>     fn yy(self) -> Vec2;
>     fn xxx(self) -> Vec3;
>     fn xxy(self) -> Vec3;
>     // ...
>     fn xxxy(self) -> Vec4;
>     fn xxyx(self) -> Vec4;
>     // ...
> }
> impl Vec2Swizzle for Vec2 {
>     fn xx(self) -> Vec2 {
>         Vec2 {
>             x: self.x,
>             y: self.x,
>         }
>     }
>     fn xy(self) -> Vec2 {
>         Vec2 {
>             x: self.x,
>             y: self.y,
>         }
>     }
>     // ...
> }
> trait Vec3Swizzle {
>     fn xx(self) -> Vec2;
>     fn xy(self) -> Vec2;
>     fn xz(self) -> Vec2;
>     // ...
> }
> // ... (4k+ lines total)
> ```

</details>


<details open>
<summary>

# More Examples
</summary>

## Generating data
```rust
// Use the `f32` function and alike to make a post-fixed number literal.
// (This is a contrived example for demonstration, as Rust std already have the `f32::GOLDEN_RATIO` constant)
let GOLDEN_RATIO = pymeta!($f32((1 + 5 ** 0.5) / 2)$);

// Expansion:
1.618034f32
```
```rust
pymeta! {
    INCLUDE!("BACKEND_TEST/include/sin_table.input.rs")
}
// or with numpy:
pymeta! {
    INCLUDE!("BACKEND_TEST/include/sin_table_np.input.rs")
}

// Expansion:
const SIN_TABLE: [f32; 256] = [0.0, 0.024541228522912288, /*...*/ -0.04906767432741809, -0.024541228522912448];
```

## Semi-quoting
```rust
pymeta! {
    INCLUDE!("BACKEND_TEST/include/semi_quoting_intro.input.rs")
}

// Expansion:
INCLUDE!("BACKEND_TEST/include/semi_quoting_intro.output.rs")
```

## Pure-Python code blocks
When writing many Python statements in PyMeta, adding the `$` symbols on every line could become annoying.<br>
The `${...}` syntax could be used to create a "pure-Python" block:
```rust
pymeta! {
    ${ // Pure-Python block
        // Semicolon and braces are still required.
        import numpy;
        N = 10;
        for i in range(N):{
            ...
        }
    }
    
    // You can also make a Python indent block be a pure-Python block (note the `:${` part):
    $while True:${
        x = foo(y + z);
        if a == b:{
            break;
        }
    }
}
```

## Defining simple PyMeta macros (`#[pymeta_func]`)
`#[pymeta_func]` literally defines a Python function "as" a Rust macro.<br>
You can invoke such a Rust macro like calling the Python function.<br>
The function's return value will becomes the expansion result of the Rust macro.
```rust
// A simple example.
#[pymeta_func(a: int, b: int, c: int)] // This is the function parameter list (types are not necessary).
#[public(crate)] // This specifies the visibility (like `pub(crate)` in Rust), you can also specify other visibilities such as `#[public(super)]`.
fma! { // `fma` is the name of the function and the Rust macro.
    // The function body is a pure-Python block, so no `$`s are needed.
    return a * b + c;
}
// Usage:
fn main() {
    let num = fma!(2, 3, 4); // Expansion: 10 (2*3+4)
    println!("2*3+4={num}");
}
```
```rust
// A more complex example.
/// Creates a sorted array of (key, value) pairs at compile time.
/// Optionally sort by the `key` function.
/// This is useful for creating an id registry-table that can be bisected.
#[pymeta_func(INCLUDE!("BACKEND_TEST/include/func_sorted_array.param.rs"))]
INCLUDE!("BACKEND_TEST/include/func_sorted_array.input.rs")
// Usage:
sorted_array!("ENTITY_REGISTRY", {{ (u16, &'static dyn GameEntityType) }}, {
    2: "SheepEntityType",
    1: "PigEntityType",
    1000: "PlayerEntityType",
    100: {{ ZombieEntityType::new(ZombieType::Zombie) }},
    101: {{ ZombieEntityType::new(ZombieType::Husk) }},
});
// Expansion:
const ENTITY_REGISTRY: [(u16, &'static dyn GameEntityType); 5] = [
    (1, PigEntityType),
    (2, SheepEntityType),
    (100, ZombieEntityType::new(ZombieType::Zombie)),
    (101, ZombieEntityType::new(ZombieType::Husk)),
    (1000, PlayerEntityType),
];
```
Note to RustRover users: due to an IDE [bug](https://youtrack.jetbrains.com/projects/RUST/issues/RUST-20689/Unexpected-merging-of-spanned-identifiers-in-output-of-proc-macro),
the IDE may fail to expand a `pymeta_func` "invoke" if it contains `$` symbols.
A workaround for now is to put spaces around the `$` symbols.

## Using data from external files
```rust
#[pymeta_func(name: str)]
#[public(crate)]
item_id! {
    import os;
    import json;
    from pathlib import Path;
    
    // You can get the path (Python `pathlib.Path`) to the Rust file that called the macro through `Span.call_site()`.
    // This can be used to achieve a similar effect as the builtin `include!()` macro:
    dir = Span.call_site().local_file().parent;
    // However, IDE support for `Span.local_file()` is not great currently.
    // So currently it's suggested to use a path relative to the cargo package root instead,
    // using the `CARGO_MANIFEST_DIR` environment variable to get the absolute path to the package root.
    dir = Path(os.getenv("CARGO_MANIFEST_DIR"));
    
    file = dir / "data/items" / f~"{name}.json";
    
    // The compiler caches macro expansions,
    // so changes made to external files may not take effect until you do a `cargo clean`.
    // If you are on nightly Rust, with PyMeta's `nightly_tracked` feature enabled,
    // you can inform Rust that the macro expansion depend on some external file:
    pymeta.track_path(file);
    // If you are on stable, unfortunately you may have to run `cargo clean`
    // for the changes in external files to take effect.
    
    // Reminder: the `u16` function and others alike can be used to create a post-fixed number literal.
    return u16(json.load(open(file))["id"]);
}

const ARROW_ID: u16 = item_id!("arrow");

// $CARGO_MANIFEST_DIR/data/items/arrow.json: {"id": 42, ...}
// Expansion:
42u16
```

## Reusing PyMeta code (`#[pymeta_module]`)
`#[pymeta_module]` allows you to "embed" Python modules in Rust modules.<br>
Allowing you to share common metaprogramming code and data.
```rust
mod py_utils {
    use pymeta::*;

    #[pymeta_module]
    #[public(crate)]
    common! {
        def entity_struct_name(name: str):{
            return name + "Entity";
        }

        ENTITY_COMMON_FIELDS = [
            ("health", "f32"),
            ("position", "Vec3"),
        ];
    }
}

// Usage
pymeta! {
    // Use `import!` to import from PyMeta modules.
    $import! py_utils::common;
    // You can also import specific items from the module.
    $import! py_utils::common.entity_struct_name;
    // The following syntax are also supported:
    // $import! py_utils::common as alias;
    // $import! py_utils::common.{self, a, b as c};
    
    struct $entity_struct_name("Cat")$ {
        $for field,typ in common.ENTITY_COMMON_FIELDS:{
            $field$ : $typ$,
        }
        cat_type: CatType,
    }
}
// Expansion:
struct CatEntity {
    health: f32,
    position: Vec3,
    cat_type: CatType,
}
```


</details>


<details>
<summary>

# *Cursed Examples*

</summary>

This section contains ~~obviously cursed~~ fun examples that you should probably not use in actual projects.
<br>
That said, they do include examples of some useful advanced features of PyMeta that are not yet documented elsewhere.

```rust
pymeta! {
    // Include Rust code straight from the Internet!
    // *Who needs cargo when you have this?*
    $from urllib import request;
    $URL = "https://raw.githubusercontent.com/shBLOCK/pymeta/cecb0a1/pymeta-proc-macro-backend/src/utils/rust_token.rs";
    // `Tokens.parse()` parses string into Rust code.
    $Tokens.parse(request.urlopen(URL).read().decode())$
}
```

```rust
// Inspiration: https://jon.how/likepython/
#[pymeta_func(input_tokens: Tokens)]
like_rust! {
    WORDS = {"so", "like", "right", "totally", "something", "dude", "bro", "man", "just", "yo", "lol", "yeah", "uh", "um", "ah", "plz", "that", "or", "and", "then", "first", "things", "damn", "this", "thing"};

    def process(tokens: Tokens):{
        for token in tokens:{
            match token:{
                case Ident(string) if string.lower() in WORDS:{ continue; }
                // Recurse into groups
                case Group() as group:{
                    group.tokens = Tokens(items=process(group.tokens));
                }
            }
            yield token;
        }
    }

    // `Tokens(items=...)` expects an iterable of `Token`s.
    return Tokens(items=process(input_tokens));
}

// Actual Python-based proc-macros will be implemented in the future,
// which would allow passing in arbitrary Rust tokens directly.
// For now, the semi-quoting syntax can be used for passing in tokens to a Python macro.
like_rust! { {{
    yeah fn this main() and then {
        uh so like for i in 0..16 or something {
            then just match that i {
                right first things first _ if i % 3 == 0 && i % 5 == 0 => then just totally println!("FizzBuzz") yo,
                then like _ if i % 3 == 0 => just println!("Fizz") plz,
                and yeah _ if i % 5 == 0 => just println!("Buzz") bro,
                uh and _ => then dude just println!(that damn "{i}") lol
            }
        }
    }
}} }

// Expansion:
fn main() {
    for i in 0..16 {
        match i {
            _ if i % 3 == 0 && i % 5 == 0 => println!("FizzBuzz"),
            _ if i % 3 == 0 => println!("Fizz"),
            _ if i % 5 == 0 => println!("Buzz"),
            _ => println!("{i}"),
        }
    }
}
```

```rust
// *No I'm not vibe-coding, the Rust compiler is!*
#[pymeta_func(prompt: str)]
vibe! {
    from openai import OpenAI;
    client = OpenAI(base_url="http://127.0.0.1:52625/v1", api_key="");

    response = client.chat.completions.create(
        model="qwen3-it:4b",
        messages=[
            {"role": "system", "content": "You are a Rust code generator. Generate Rust code according to user prompt. "
                                          +"Please ONLY generate Rust code in plain text, no explantations and other natural language. "
                                          +"DO NOT GENERATE ANY COMMENTS (including doc comments)! "
                                          +"Always output some code, even if the prompt is not clear or you think there's a problem with the prompt."
            },
            {"role": "user", "content": prompt}
        ],
        seed=0, // remove this line for extra vibes
        stream=True
    );

    result = [];
    for chunk in response:{
        chunk = chunk.choices[0].delta.content;
        if chunk:{
            result.append(chunk);
            print(chunk, end="", flush=True);
        }
    }
    result = "".join(result);

    return Tokens.parse("\n".join(line for line in result.splitlines() if not line.startswith("```")));
}

vibe!("Gimme Vec2, 3 and 4 structs with some helpful methods PLS!");
```

<details>
<summary>Macro expansion</summary>

```rust
#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::zero()
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        }
    }
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
    pub fn subtract(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
    pub fn multiply_scalar(&self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
    pub fn magnitude(&self) -> f64 {
        self.length()
    }
    pub fn distance_to(&self, other: &Self) -> f64 {
        self.subtract(other).length()
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::zero()
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        }
    }
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
    pub fn subtract(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
    pub fn multiply_scalar(&self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
    pub fn magnitude(&self) -> f64 {
        self.length()
    }
    pub fn distance_to(&self, other: &Self) -> f64 {
        self.subtract(other).length()
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Vec4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}
impl Vec4 {
    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::zero()
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
                w: self.w / len,
            }
        }
    }
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
    pub fn subtract(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
    pub fn multiply_scalar(&self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
    pub fn magnitude(&self) -> f64 {
        self.length()
    }
    pub fn distance_to(&self, other: &Self) -> f64 {
        self.subtract(other).length()
    }
}
```

</details>

</details>

<details open>
<summary>

# Advanced Usage
</summary>

Many of PyMeta's features are still undocumented at this point,
especially the Python APIs of the `pymeta` Python module.<br>
(Many of the objects and functions, such as `Tokens`, `Punct`, `lit()` and `f32()` also come from the `pymeta` module.)<br>

The most important parts has been explained in the examples,
but for more advanced usage, like constructing arbitrary tokens programmatically or parsing them,
more of the Python API is needed.<br>
If you are feeling adventurous, take a look at the **Python source code of the `pymeta` module: [pymeta-proc-macro-backend/pylib/src/pymeta/\_\_init\_\_.py](pymeta-proc-macro-backend/pylib/src/pymeta/__init__.py)**

</details>

# Attributions
- The [repetitive](https://github.com/Noam2Stein/repetitive) crate: initial inspiration for the "inline metaprogramming" syntax.<br>
  Check it out if you want to do simple metaprogramming in a embedded Rust-like language instead of Python!
- The [ct_python](https://docs.rs/ct-python) crate: inspiration for running Python at compile time in a proc-macro.
- The [PyO3](https://pyo3.rs/) project: a major part of this project, without which PyMeta would not be possible.
