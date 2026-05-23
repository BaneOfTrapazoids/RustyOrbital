@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

const a_0: f32 = 0.00000000005291772228743774064696481;
const n: u32 = 1;
const l: u32 = 2;
const m: i32 = 0;

@compute
@workgroup_size(10, 10, 10)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    let total = arrayLength(&input);

    if (index >= total) {
        return;
    }

    let x = f32(id.x) / 50 - 1.0;
    let y = f32(id.y) / 50 - 1.0;
    let z = f32(id.z) / 50 - 1.0;

    // a simple copy operation
    output[id.x+id.y*100+id.z*10000] = u32(x * x + y * y + z * z < 1);
}