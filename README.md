PyMeta - Metaprogramming Rust in Python!
---

**PyMeta** offers proc-macros to use **Python** as a metaprogramming language for Rust.

Currently, only CPython via PyO3 is supported, so a Python interpreter needs to be installed for this crate to work.
<br>
For now, Python >=3.14 is needed, this requirement will be relaxed to allow older Python versions soon.
<br>
See [PyO3's documentation on configuring the Python version](https://pyo3.rs/v0.28.0/building-and-distribution.html#configuring-the-python-version).

Nightly Rust is required for now. Nightly features will be made gated behind feature flags and/or polyfilled from stable soon.

# Features

- Generate & manipulate any arbitrary Rust code from Python
- Write metaprogramming Python **alongside** normal Rust code
    - Control code generation using any **Python control flow** (`for`, `if`, `match`, `with`, functions, etc.)
- Support multiple **Python implementations**:
    - [Official CPython](https://www.python.org) (via [PyO3](https://pyo3.rs)) - Full Python ecosystem support (e.g.
      numpy, pandas)
    - 🛠️ Coming soon: [RustPython](https://rustpython.github.io) - Pure Rust
    - 🛠️ Planned: [MicroPython](https://micropython.org) - Minimal bloat, fast compile time
- Dev experience & IDE integration:
    - Preserves all `Span` (**source location**) information throughout the entire code generation pipeline
      - **Trace back** from generate code to the Python expression that generated it
      - Report Python exceptions with **traceback** at precise Rust source code locations
- 🛠️ Planned: Writing custom macros and proc-macros in Python
- 🛠️ Planned: Reusing code: Importing/exporting PyMeta Python modules from/to other Rust modules


<details open>
<summary>

# Intro Example: A Vector Math Module

</summary>

In this example, we will use PyMeta to implement part of a [GLM](https://github.com/g-truc/glm)-like vector math module.

Let's start by defining our vector structs.

```rust
pymeta! {
    // `$` denotes the start of some Python code.
    // Unlike normal Python, we need to use braces to identify a code block.
    $for dims in range(2, 5):{
        // Normal Rust code can be written alongside metaprogramming Python code.
        #[derive(Clone, Copy, Debug, PartialEq)]
        // Two `$`s denote an inline Python expression, the value of which will be converted into Rust code.
        // `~` is the "concat marker", we use it here so we get `Vec2`, not `Vec 2`.
        struct Vec~$dims$ {
            $for i in range(dims):{
                // Here, again, two `$`s denote an inline Python expression.
                $"xyzw"[i]$: f32,
            }
        }
    }
}

// Macro expansion:
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec2 {
    x: f32,
    y: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}
```

If you put the code above in an IDE or compile it,
notice how the compiler and IDE knows that `Vec2`, `Vec3` and `Vec4` comes from `Vec~$dims$`!
<br>
The compiler will report that the 3 struct are unused while pointing out `Vec~$dims$`,
and your IDE may show `Vec~$dims$` in a gray color.

Next, let's implement some binary operation traits for our vector structs.

```rust
pymeta! {
    // We can write Python statements and define Python variables.
    $BINARY_OPS = [
        ("Add", "+"),
        ("Sub", "-"),
        ("Mul", "*"),
        ("Div", "/"),
        ("Rem", "%"),
    ]; // Remember to terminate a Python statement with a semicolon!

    $for dims in range(2, 5):{
        $for op_name, op_sym in BINARY_OPS:{
            $for inplace in [False, True]:{
                impl std::ops::$op_name + ("Assign" if inplace else "")$ for Vec~$dims$ {
                    // Python control flows can be used to control code generation.
                    $if not inplace:{
                        type Output = Vec~$dims$;
                    }

                    $if not inplace:{
                        fn $op_name.lower()$(self, rhs: Self) -> Self {
                            Self {
                                $for d in "xyzw"[:dims]:{
                                    $d$: self.$d$$op_sym$ rhs.$d$,
                                }
                            }
                        }
                    } $else:{
                        // Prefixed literals are reserved syntax in Rust,
                        // to work around this, `f"string"` can be written as `f~"string"`. 
                        fn $f~"{op_name.lower()}_assign"$(&mut self, rhs: Self) {
                            $for d in "xyzw"[:dims]:{
                                self.$d$$op_sym + "="$ rhs.$d$;
                            }
                        }
                    }
                }
            }
        }
    }
}

// Macro expansion:
impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
// ... (200+ more lines, see below for full expansion)
```

<details>
<summary>Full macro expansion</summary>

```rust
impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl std::ops::SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
impl std::ops::Mul for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}
impl std::ops::MulAssign for Vec2 {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}
impl std::ops::Div for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}
impl std::ops::DivAssign for Vec2 {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}
impl std::ops::Rem for Vec2 {
    type Output = Vec2;
    fn rem(self, rhs: Self) -> Self {
        Self {
            x: self.x % rhs.x,
            y: self.y % rhs.y,
        }
    }
}
impl std::ops::RemAssign for Vec2 {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
    }
}
impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}
impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}
impl std::ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}
impl std::ops::Mul for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}
impl std::ops::MulAssign for Vec3 {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}
impl std::ops::Div for Vec3 {
    type Output = Vec3;
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}
impl std::ops::DivAssign for Vec3 {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}
impl std::ops::Rem for Vec3 {
    type Output = Vec3;
    fn rem(self, rhs: Self) -> Self {
        Self {
            x: self.x % rhs.x,
            y: self.y % rhs.y,
            z: self.z % rhs.z,
        }
    }
}
impl std::ops::RemAssign for Vec3 {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
        self.z %= rhs.z;
    }
}
impl std::ops::Add for Vec4 {
    type Output = Vec4;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }
}
impl std::ops::AddAssign for Vec4 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
        self.w += rhs.w;
    }
}
impl std::ops::Sub for Vec4 {
    type Output = Vec4;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
        }
    }
}
impl std::ops::SubAssign for Vec4 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
        self.w -= rhs.w;
    }
}
impl std::ops::Mul for Vec4 {
    type Output = Vec4;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            w: self.w * rhs.w,
        }
    }
}
impl std::ops::MulAssign for Vec4 {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
        self.w *= rhs.w;
    }
}
impl std::ops::Div for Vec4 {
    type Output = Vec4;
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
            w: self.w / rhs.w,
        }
    }
}
impl std::ops::DivAssign for Vec4 {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
        self.w /= rhs.w;
    }
}
impl std::ops::Rem for Vec4 {
    type Output = Vec4;
    fn rem(self, rhs: Self) -> Self {
        Self {
            x: self.x % rhs.x,
            y: self.y % rhs.y,
            z: self.z % rhs.z,
            w: self.w % rhs.w,
        }
    }
}
impl std::ops::RemAssign for Vec4 {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
        self.z %= rhs.z;
        self.w %= rhs.w;
    }
}
```

</details>

Then, let's add [swizzle](https://en.wikipedia.org/wiki/Swizzling_(computer_graphics)) operations to the vectors.
Since this involves a LOT of code to cover all possible arrangements, we will put them in traits and implement them for
our vectors,
to improve compile times.

```rust
pymeta! {
    // Take advantage of all the Python modules!
    $import itertools;

    $for in_dims in range(2, 5):{
        // The `rust()` function coerce its inputs to Rust code and
        // emit (append) them into the currently active `Tokens` context (more about this later).
        // If the last token is a "group" (`()`, `[]`, or `{}`), the `Group` object returned out so we can populate them later.
        // Here we store the returned `Group`s representing the body of our `trait` and `impl`.
        $trait_body = rust(f~"trait Vec{in_dims}Swizzle {{}}");
        $impl_body = rust(f~"impl Vec{in_dims}Swizzle for Vec{in_dims} {{}}");

        $for out_dims in range(2, 5):{
            $out_name = f~"Vec{out_dims}";
            // Use the product function from the itertools module we imported earlier to generate swizzle arrangements.
            $for swizzle in itertools.product(*(["xyzw"[:in_dims]] * out_dims)):{
                // Use the `with` statement to temporarily set a `Tokens` object as the current context.
                // This means code emitted from within the `with` block are added to that `Tokens` object.
                // In this case, `trait_body` and `impl_body` are actually `Group` objects,
                // using `with` on a `Group` is a shorthand for `with group.tokens`.
                $with trait_body:{
                    fn $"".join(swizzle)$(self) -> $out_name$;
                }
                $with impl_body:{
                    fn $"".join(swizzle)$(self) -> $out_name$ {
                        $out_name$ {
                            $for a, b in zip("xyzw", swizzle):{
                                $a$: self.$b$,
                            }
                        }
                    }
                }
            }
        }
    }
}

// Macro expansion:
trait Vec2Swizzle {
    fn xx(self) -> Vec2;
    fn xy(self) -> Vec2;
    fn yx(self) -> Vec2;
    fn yy(self) -> Vec2;
    fn xxx(self) -> Vec3;
    fn xxy(self) -> Vec3;
    fn xyx(self) -> Vec3;
    fn xyy(self) -> Vec3;
    fn yxx(self) -> Vec3;
    fn yxy(self) -> Vec3;
    fn yyx(self) -> Vec3;
    fn yyy(self) -> Vec3;
    fn xxxx(self) -> Vec4;
    fn xxxy(self) -> Vec4;
    fn xxyx(self) -> Vec4;
    fn xxyy(self) -> Vec4;
    fn xyxx(self) -> Vec4;
    fn xyxy(self) -> Vec4;
    fn xyyx(self) -> Vec4;
    fn xyyy(self) -> Vec4;
    fn yxxx(self) -> Vec4;
    fn yxxy(self) -> Vec4;
    fn yxyx(self) -> Vec4;
    fn yxyy(self) -> Vec4;
    fn yyxx(self) -> Vec4;
    fn yyxy(self) -> Vec4;
    fn yyyx(self) -> Vec4;
    fn yyyy(self) -> Vec4;
}
impl Vec2Swizzle for Vec2 {
    fn xx(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.x,
        }
    }
    fn xy(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }
    // ...
}
trait Vec3Swizzle {
    fn xx(self) -> Vec2;
    fn xy(self) -> Vec2;
    fn xz(self) -> Vec2;
    fn yx(self) -> Vec2;
    fn yy(self) -> Vec2;
    fn yz(self) -> Vec2;
    fn zx(self) -> Vec2;
    fn zy(self) -> Vec2;
    fn zz(self) -> Vec2;
    fn xxx(self) -> Vec3;
    fn xxy(self) -> Vec3;
    fn xxz(self) -> Vec3;
    fn xyx(self) -> Vec3;
    // ...
}
// ... (4k+ more lines)
```

</details>


<details open>
<summary>

# Examples

</summary>

## Build Metadata
```rust
// Note that due to rustc's caching, this may not update on every build,
// unless you do a `cargo clean` beforehand.
const BUILD_TIME: &str = pymeta! {
    $from datetime import datetime;
    // By default, a string is turned into Rust code by parsing their content.
    // Here we use `lit()` to make a string literal instead.
    $lit(str(datetime.now()))$
};

// Macro expansion:
"2026-02-16 14:38:05.633646"
```

## Include data from external file
```rust
// Better ways to define custom macros and even proc-macros using Python will be added in the future.
// For now, the `macro_metavar_expr` nightly feature is required.
#![feature(macro_metavar_expr)]
macro_rules! module_id {
    ($name:literal) => {
        pymeta::pymeta! {
            $$import json;
            $$name = $name;
            // TODO: The working directory of the macro is currently not defined and may not be consistent.
            // This will be improved in the future, but for now, the CWD is most likely the project root.
            $$json.load(open(f~"examples/{name}.json"))["id"]$$
        }
    };
}

const FOO_MODULE_ID: u32 = module_id!("foo");

// Macro expansion:
42
```

## Generating Data
```rust
// Use the `f32` function and alike to make a post-fixed number literal.
let GOLDEN_RATIO = pymeta!($f32((1 + 5 ** 0.5) / 2)$);

// Macro expansion:
1.618034f32
```
```rust
pymeta! {
    $from math import *;
    $N = 256;
    // `Token.join()` works like `str.join()`.
    const SIN_TABLE: [f32; $N$] = [$Punct(',').join(sin(i / N * tau) for i in range(N))$];
}
// or with numpy:
pymeta! {
    $import numpy as np;
    $from math import tau;
    $N = 256;
    const SIN_TABLE: [f32; $N$] = [$Punct(',').join(np.sin(np.linspace(0, tau, N, endpoint=False)))$];
}

// Macro expansion:
const SIN_TABLE: [f32; 256] = [0.0, 0.024541228522912288, 0.049067674327418015, 0.07356456359966743, 0.0980171403295606, 0.1224106751992162, 0.14673047445536175, 0.17096188876030122, 0.19509032201612825, 0.2191012401568698, 0.24298017990326387, 0.26671275747489837, 0.29028467725446233, 0.3136817403988915, 0.33688985339222005, 0.3598950365349881, 0.3826834323650898, 0.40524131400498986, 0.4275550934302821, 0.44961132965460654, 0.47139673682599764, 0.49289819222978404, 0.5141027441932217, 0.5349976198870972, 0.5555702330196022, 0.5758081914178453, 0.5956993044924334, 0.6152315905806268, 0.6343932841636455, 0.6531728429537768, 0.6715589548470183, 0.6895405447370668, 0.7071067811865476, 0.7242470829514669, 0.7409511253549591, 0.7572088465064846, 0.7730104533627369, 0.7883464276266062, 0.8032075314806448, 0.8175848131515837, 0.8314696123025452, 0.844853565249707, 0.8577286100002721, 0.8700869911087113, 0.8819212643483549, 0.8932243011955153, 0.9039892931234433, 0.9142097557035307, 0.9238795325112867, 0.9329927988347388, 0.9415440651830208, 0.9495281805930367, 0.9569403357322089, 0.9637760657954398, 0.970031253194544, 0.9757021300385286, 0.9807852804032304, 0.9852776423889412, 0.989176509964781, 0.99247953459871, 0.9951847266721968, 0.9972904566786902, 0.9987954562051724, 0.9996988186962042, 1.0, 0.9996988186962042, 0.9987954562051724, 0.9972904566786902, 0.9951847266721969, 0.99247953459871, 0.989176509964781, 0.9852776423889412, 0.9807852804032304, 0.9757021300385286, 0.970031253194544, 0.9637760657954398, 0.9569403357322089, 0.9495281805930367, 0.9415440651830208, 0.9329927988347388, 0.9238795325112867, 0.9142097557035307, 0.9039892931234434, 0.8932243011955152, 0.881921264348355, 0.8700869911087115, 0.8577286100002721, 0.8448535652497072, 0.8314696123025453, 0.8175848131515837, 0.8032075314806449, 0.7883464276266063, 0.7730104533627371, 0.7572088465064847, 0.740951125354959, 0.7242470829514669, 0.7071067811865476, 0.689540544737067, 0.6715589548470186, 0.6531728429537766, 0.6343932841636455, 0.6152315905806269, 0.5956993044924335, 0.5758081914178454, 0.5555702330196022, 0.5349976198870972, 0.5141027441932218, 0.49289819222978415, 0.4713967368259978, 0.4496113296546069, 0.42755509343028203, 0.4052413140049899, 0.3826834323650899, 0.35989503653498833, 0.33688985339222033, 0.3136817403988914, 0.2902846772544624, 0.2667127574748985, 0.24298017990326407, 0.21910124015687005, 0.1950903220161286, 0.17096188876030122, 0.1467304744553618, 0.12241067519921635, 0.09801714032956083, 0.07356456359966773, 0.049067674327417966, 0.024541228522912326, 0.00000000000000012246467991473532, -0.02454122852291208, -0.049067674327417724, -0.0735645635996675, -0.09801714032956059, -0.1224106751992161, -0.14673047445536158, -0.17096188876030097, -0.19509032201612836, -0.2191012401568698, -0.24298017990326382, -0.26671275747489825, -0.2902846772544621, -0.3136817403988912, -0.3368898533922201, -0.3598950365349881, -0.38268343236508967, -0.4052413140049897, -0.4275550934302818, -0.44961132965460665, -0.47139673682599764, -0.4928981922297839, -0.5141027441932216, -0.5349976198870969, -0.555570233019602, -0.5758081914178453, -0.5956993044924332, -0.6152315905806267, -0.6343932841636453, -0.6531728429537765, -0.6715589548470184, -0.6895405447370668, -0.7071067811865475, -0.7242470829514668, -0.7409511253549589, -0.7572088465064842, -0.7730104533627367, -0.7883464276266059, -0.803207531480645, -0.8175848131515838, -0.8314696123025452, -0.8448535652497071, -0.857728610000272, -0.8700869911087113, -0.8819212643483549, -0.8932243011955152, -0.9039892931234431, -0.9142097557035305, -0.9238795325112865, -0.932992798834739, -0.9415440651830208, -0.9495281805930367, -0.9569403357322088, -0.9637760657954398, -0.970031253194544, -0.9757021300385285, -0.9807852804032303, -0.9852776423889411, -0.9891765099647809, -0.9924795345987101, -0.9951847266721969, -0.9972904566786902, -0.9987954562051724, -0.9996988186962042, -1.0, -0.9996988186962042, -0.9987954562051724, -0.9972904566786902, -0.9951847266721969, -0.9924795345987101, -0.9891765099647809, -0.9852776423889412, -0.9807852804032304, -0.9757021300385286, -0.970031253194544, -0.96377606579544, -0.9569403357322089, -0.9495281805930368, -0.9415440651830209, -0.9329927988347391, -0.9238795325112866, -0.9142097557035306, -0.9039892931234433, -0.8932243011955153, -0.881921264348355, -0.8700869911087115, -0.8577286100002722, -0.8448535652497072, -0.8314696123025455, -0.8175848131515839, -0.8032075314806453, -0.7883464276266061, -0.7730104533627369, -0.7572088465064846, -0.7409511253549591, -0.724247082951467, -0.7071067811865477, -0.6895405447370672, -0.6715589548470187, -0.6531728429537771, -0.6343932841636459, -0.6152315905806274, -0.5956993044924332, -0.5758081914178452, -0.5555702330196022, -0.5349976198870973, -0.5141027441932219, -0.49289819222978426, -0.4713967368259979, -0.449611329654607, -0.42755509343028253, -0.4052413140049904, -0.3826834323650904, -0.359895036534988, -0.33688985339222, -0.3136817403988915, -0.2902846772544625, -0.2667127574748986, -0.24298017990326418, -0.21910124015687016, -0.19509032201612872, -0.17096188876030177, -0.1467304744553624, -0.12241067519921603, -0.0980171403295605, -0.07356456359966741, -0.04906767432741809, -0.024541228522912448];
```

## Semi-quoting
```rust
pymeta! {
    // The `Tokens` class can be used for semi-quoting.
    // (A dedicated semi-quoting expression syntax may be added in the future.)
    $with Tokens() as signiture:{ fn say_hello(name: &str) }
    
    trait Hello {
        $signiture$;
    }
    
    struct MyStruct;
    impl Hello for MyStruct {
        $signiture$ {
            println!("Hello {name}!");
        }
    }
}

// Macro expansion:
trait Hello {
    fn say_hello(name: &str);
}
struct MyStruct;
impl Hello for MyStruct {
    fn say_hello(name: &str) {
        println!("Hello {name}!");
    }
}
```

</details>


<details>
<summary>

# ~~Cursed Examples~~

</summary>

This section contains ~~obviously cursed~~ fun examples that you should probably not use in actual projects.
<br>
That said, they do a good job at demonstrating the flexibility of PyMeta.

```rust
pymeta! {
    // Include Rust code straight from the Internet!
    // *Who needs cargo when you have this?*
    $from urllib import request;
    $URL = "https://raw.githubusercontent.com/shBLOCK/pymeta/refs/heads/main/src/utils/rust_token.rs";
    $request.urlopen(URL).read().decode()$
}

// Macro expansion:
// Well, basically this... : https://raw.githubusercontent.com/shBLOCK/pymeta/refs/heads/main/src/utils/rust_token.rs
```

```rust
#![feature(macro_metavar_expr)]

// Inspiration: https://jon.how/likepython/
macro_rules! like_rust {
    // Proper support for Python-based proc-macro will be added in the future.
    // For now, semi-quoting using `with Token():` can achieve a similar effect as a custom proc-macro.
    ($($input:tt)*) => {
        pymeta::pymeta! {
            $$with Tokens() as input:{
                $($input)*
            }

            $$WORDS = {"so", "like", "right", "totally", "something", "dude", "bro", "man", "just", "yo", "lol", "yeah", "uh", "um", "ah", "plz", "that", "or", "and", "then", "first", "things", "damn", "this", "thing"};

            $$def process(tokens: Tokens):{
                $$for token in tokens:{
                    $$if isinstance(token, Ident) and token.string.lower() in WORDS:{
                        $$continue;
                    }
                    $$if isinstance(token, Group):{
                        $$token.tokens = Tokens(items=process(token.tokens));
                    }
                    $$yield token;
                }
            }

            $$Tokens(items=process(input))$$
        }
    };
}

like_rust! {
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
}

// Macro expansion:
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
#![feature(macro_metavar_expr)]

// *No I'm not vibe-coding, the Rust compiler is!*
macro_rules! vibe {
    ($prompt:tt) => {
        pymeta::pymeta! {
            $$from openai import OpenAI;
            $$client = OpenAI(base_url="http://127.0.0.1:52625/v1", api_key="");

            $$response = client.chat.completions.create(
                model="qwen3-it:4b",
                messages=[
                    {"role": "system", "content": "You are a Rust code generator. Generate Rust code according to user prompt. "
                                                  +"Please ONLY generate Rust code in plain text, no explantations and other natural language. "
                                                  +"DO NOT GENERATE ANY COMMENTS (including doc comments)! "
                                                  +"Always output some code, even if the prompt is not clear or you think there's a problem with the prompt."
                    },
                    {"role": "user", "content": $prompt}
                ],
                seed=0, // remove this line for extra vibes
                stream=True
            );

            $$result = [];
            $$for chunk in response:{
                $$chunk = chunk.choices[0].delta.content;
                $$if chunk:{
                    $$result.append(chunk);
                    $$print(chunk, end="", flush=True);
                }
            }
            $$result = "".join(result);

            $$"\n".join(line for line in result.splitlines() if not line.startswith("```"))$$
        }
    };
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


# Attributions

This crate is inspired by the great [repetitive](https://github.com/Noam2Stein/repetitive) crate
by [Noam2Stein](https://github.com/Noam2Stein).
<br>
Check it out if you want to do metaprogramming in a Rust-like language instead of Python!