# Introduction
This is a WIP documentation for PyMeta.
For now, please checkout [docs.rs](https://docs.rs/pymeta/latest/pymeta/) instead.

In this example, we will use PyMeta to implement part of a [GLM](https://github.com/g-truc/glm)-like vector math module.

Let's start by defining our vector structs.

```pymeta
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

```pymeta
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