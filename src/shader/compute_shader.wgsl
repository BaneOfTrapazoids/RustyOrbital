// A read-only storage buffer that stores and array of unsigned 32bit integers
@group(0) @binding(0) var<storage, read> input: array<u32>;
// This storage buffer can be read from and written to
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

const a_0: f32 = 0.00000000005291772228743774064696481;
const n: u32 = 1;
const l: u32 = 2;
const m: i32 = 0;

// Tells wgpu that this function is a valid compute pipeline entry_point
@compute
// Specifies the "dimension" of this work group
@workgroup_size(10, 10, 10)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    let total = arrayLength(&input);

    // workgroup_size may not be a multiple of the array size so
    // we need to exit out a thread that would index out of bounds.
    if (index >= total) {
        return;
    }

    // a simple copy operation
    output[id.x+id.y*100+id.z*10000] = u32(f32(id.x * id.x + id.y * id.y + id.z * id.z) / 1000 < 0.1);
}