@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

const a_0: f32 = 0.00000000005291772228743774064696481;
const a_0_star: f32 = 0.000000000052946541;
const n: f32 = 4;
const l: f32 = 3;
const m: f32 = 1;
const k_scale = 200*a_0;
const PI: f32 = 3.14159265358979323846264338327950288;

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


    let r = length(vec3<f32>(x * k_scale, y * k_scale, z * k_scale));
    let rho = 2 * r / (n * a_0_star);
//    let theta = acos(z / length(vec3<f32>(x, y, z)));
    let theta = acos(z * k_scale / r);

    // a simple copy operation
    output[id.x+id.y*100+id.z*10000] = u32(pow(abs((sqrt(rho / r * rho / r * rho / r) * factorial(n-l - 1) / (2*n * factorial(n+l))) * exp(-rho / 2) * pow(rho, l)), 2.0) * lag(2*l+1, n-l - 1, rho) * harmonic(m, l, theta) > 0.45);
}

fn factorial(num: f32) -> f32 {
    if(num < 0) {
        return 1;
    }
    var k = num;
    for(var i: f32 = 2.0; i < num; i += 1.0) {
        k *= i;
    }
    return max(k, 1.0);
}

fn lag(alpha: f32, deg: f32, x: f32) -> f32 {
    var l_0: f32 = 1;
    var l_1: f32 = 1 + alpha - x;
    if(deg == 0.0) {
        return l_0;
    } else if(deg == 1.0) {
        return l_1;
    }
    for(var k = 2.0; k <= deg; k += 1.0) {
        let l_n: f32 = ((2*k - 1 + alpha - x) * l_1 - (k - 1 + alpha) * l_0) / k;
        l_0 = l_1;
        l_1 = l_n;
    }

    return l_1 * l_1;
}

fn harmonic(ord: f32, deg: f32, theta: f32) -> f32 {
    return abs((2*deg+1) * factorial(deg - ord) / (4 * PI * factorial(deg + ord))) * assoc_leg(ord, deg, cos(theta));
}

fn pow_fix(a: f32, b: f32) -> f32 {
    if(b == 0) {
        return 1;
    } else if(a == 0) {
        return 0;
    }
    let x: f32 = pow(abs(a), b);
    if(a < 0 & ((i32(b) & 1) == 1)) {
        return -x;
    }
    return x;
}


fn nCr(a: f32, b: f32) -> f32 {
    if(a < 0 || b < 0) {
        return 0;
    }
    return factorial(a) / (factorial(b) * factorial(a - b));
}

fn assoc_leg(ord: f32, deg: f32, x: f32) -> f32 {
    var polynomial: f32 = 0;
    for(var k = ord; k <= deg; k += 1) {
        polynomial += factorial(k) / factorial(k - ord) * nCr(l, k) * nCr((l + k - 1) / 2, l) * pow_fix(x, k-ord);
    }
    return abs(exp2(2*deg) * pow_fix(1 - x*x, ord) * polynomial * polynomial);
}