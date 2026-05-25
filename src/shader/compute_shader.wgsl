@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<f32>;

const a_0: f32 = 0.00000000005291772228743774064696481;
const a_0_star: f32 = 0.000000000052946541;
const PI: f32 = 3.14159265358979323846264338327950288;
const radii_arr = array<f32, 16>(5, 15, 40, 45, 70, 100, 130, 160, 200, 250, 300, 350, 400, 450, 700, 900);

@compute
@workgroup_size(10, 10, 10)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    let total = arrayLength(&input);

    if (index >= total) {
        return;
    }

    let n = params[0];
    let l = params[1];
    let m = params[2];
    let radii = radii_arr[u32(n - 1)];

    let x = radii * a_0 * (f32(id.x) / 50 - 1.0);
    let y = radii * a_0 * (f32(id.y) / 50 - 1.0);
    let z = radii * a_0 * (f32(id.z) / 50 - 1.0);


    let r = length(vec3<f32>(x, y, z));
    let rho = 2 * r / (n * a_0_star);
    let theta = acos(z / r);
    let phi = atan2(y, x);
    //let theta = acos(z * k_scale / r);

    let normalization =  4 * PI * r * r * a_0 * abs((8.0 / (n * n * n * a_0_star * a_0_star * a_0_star)) * factorial(n-l - 1.0) / (2.0*n * factorial(n+l)));
    let radial = exp(-rho) * pow(rho, 2.0*l) * lag(2.0*l+1.0, n-l - 1.0, rho);
    let angular = real_harmonic(m, l, theta, phi);

    // a simple copy operation
    output[id.x+id.y*100+id.z*10000] = normalization * radial * angular;
}

fn factorial(num: f32) -> f32 {
    if(num <= 1.0) {
        return 1.0;
    }
    var k = num;
    for(var i: f32 = 2.0; i < num; i += 1.0) {
        k *= i;
    }
    return k;
}

fn factorial_fall(a: f32, b: f32) -> f32 {
    var k = 1.0;
    for(var i = 0.0; i < b; i += 1.0) {
        k *= (a-i);
    }
    return k;
}

fn lag(alpha: f32, deg: f32, x: f32) -> f32 {
    var polynomial: f32 = 0.0;
    for(var k: f32 = 0.0; k <= deg; k += 1.0) {
        polynomial += pow_fix(-1.0, k) * nCr(alpha + deg, deg - k) * pow_fix(x, k) / factorial(k);
    }
    return sign(polynomial) * polynomial * polynomial;
}

//fn real_harmonic(ord: f32, deg: f32, theta: f32, phi: f32) -> f32 {
//    if(ord < 0.0) {
//        return 2.0 * harmonic(-ord, deg, theta, phi).x;
//    } else if(ord == 0.0) {
//        return harmonic(0.0, deg, theta, phi).x;
//    } else {
//        return 2.0 * harmonic(ord, deg, theta, phi).y;
//    }
//}

fn real_harmonic(ord: f32, deg: f32, theta: f32, phi: f32) -> f32 {
    if(ord < 0.0) {
        return sign(sin(-ord * phi)) * 2.0 * (2.0 * deg + 1.0) * factorial(deg + ord) * assoc_leg_sq(-ord, deg, cos(theta)) * sin(-ord * phi)*sin(-ord * phi) / (4 * PI * factorial(deg - ord));
    } else if(ord == 0.0) {
        //return (2.0 * deg + 1.0) * assoc_leg_sq(0, deg, cos(theta)) / (4 * PI);
        return harmonic(0, deg, theta, phi).x;
    } else {
        return sign(cos(ord * phi)) * 2.0 * (2.0 * deg + 1.0) * factorial(deg - ord) * assoc_leg_sq(ord, deg, cos(theta)) * cos(ord * phi)*cos(ord * phi) / (4 * PI * factorial(deg + ord));
    }
}

fn harmonic(ord: f32, deg: f32, theta: f32, phi: f32) -> vec2<f32> {
    //let x = (pow_fix(-1.0, ord) * (2.0*deg+1.0) * factorial(deg - ord) / (4.0 * PI * factorial(deg + ord))) * assoc_leg_sq(ord, deg, cos(theta));
    let x = (2.0*deg+1.0) * factorial(deg - ord) * assoc_leg_sq(ord, deg, cos(theta)) / (4.0 * PI * factorial(deg + ord));
    return vec2<f32>(x * cos(2.0 * ord * phi), x * sin(2.0 * ord * phi));
}

fn pow_fix(a: f32, b: f32) -> f32 {
    if(b == 0.0) {
        return 1.0;
    } else if(a == 0.0) {
        return 0.0;
    } else if (b == 1.0) {
        return a;
    }
    let x: f32 = pow(abs(a), b);
    if(a < 0.0 && (i32(b) % 2 == 1)) {
        return -x;
    }
    return abs(x);
}


fn nCr(a: f32, b: f32) -> f32 {
    return factorial_fall(a, b) / factorial(b);
}

fn assoc_leg_sq(ord: f32, deg: f32, x: f32) -> f32 {
    var polynomial: f32 = 0.0;
    for(var k: f32 = ord; k <= deg; k += 1.0) {
        polynomial += factorial(k) * nCr(deg, k) * nCr((deg + k - 1.0) / 2.0, deg) * pow_fix(abs(x), k - ord) / factorial(k - ord);
    }
    return sign(polynomial) * exp2(2.0*deg) * pow_fix(1.0 - x*x, ord) * polynomial * polynomial;
}

//fn assoc_leg_sq(ord: f32, deg: f32, x: f32) -> f32 {
//    var polynomial: f32 = 0.0;
//    for(var k = 0.0; k <= f32(floor(deg / 2.0)); k += 1.0) {
//        polynomial += ((factorial(deg - 2 * k) * pow_fix(-1.0, k) * factorial(2.0 * deg - 2.0 * k)) / (factorial(deg - 2.0 * k - ord) * factorial(k) * factorial(deg - k) * factorial(deg - 2.0 * k))) * pow_fix(x, deg- 2.0 * k - ord);
//    }
//    return exp2(2.0*deg) * pow_fix(1.0 - x*x, ord) * polynomial * polynomial;
//}

//fn lag(alpha: f32, deg: f32, x: f32) -> f32 {
//    var l_0: f32 = 1;
//    var l_1: f32 = 1 + alpha - x;
//    if(deg == 0.0) {
//        return l_0;
//    } else if(deg == 1.0) {
//        return l_1;
//    }
//    for(var k = 2.0; k <= deg; k += 1.0) {
//        let l_n: f32 = ((2*k - 1 + alpha - x) * l_1 - (k - 1 + alpha) * l_0) / k;
//        l_0 = l_1;
//        l_1 = l_n;
//    }
//
//    return l_1 * l_1;
//}