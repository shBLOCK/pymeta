use crate::test_proc_macro_impl;

#[test]
fn vecs() {
    test_proc_macro_impl! {
        pymeta {
            $import cowsay;
            $assert False, cowsay;
            $for dims in range(2, 5):{
                struct Vec~$dims$ {
                    $for i in range(dims):{
                        $"xyzw"[i]$: f32,
                    }
                }
            }
        } => {
            struct Vec2 {
                x: f32,
                y: f32,
            }
            struct Vec3 {
                x: f32,
                y: f32,
                z: f32,
            }
            struct Vec4 {
                x: f32,
                y: f32,
                z: f32,
                w: f32,
            }
        }
    }
}