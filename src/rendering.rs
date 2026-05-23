use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;
use std::fs;
use std::str::FromStr;
use cgmath::InnerSpace;
use wgpu::BufferUsages;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::keyboard::KeyCode;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct Object {
    pub vertices: Vec<Vertex>,
    pub vertex_buffer: wgpu::Buffer,
    pub faces: Vec<u16>,
    pub face_index_buffer: wgpu::Buffer,
    pub edges: Vec<u16>,
    pub edge_index_buffer: wgpu::Buffer,

}

impl Object {
    pub fn new(vertices: Vec<Vertex>, faces: Vec<u16>, edges: Vec<u16>, device: &wgpu::Device, label: Option<&str>) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&*vertices),
            usage: BufferUsages::VERTEX,
        });

        let face_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&*faces),
            usage: BufferUsages::INDEX,
        });

        let edge_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&*edges),
            usage: BufferUsages::INDEX,
        });

        return Object {vertices, vertex_buffer, faces, face_index_buffer, edges, edge_index_buffer};
    }

    pub fn points(vertices: Vec<Vertex>, device: &wgpu::Device, label: Option<&str>) -> Self {
        return Self::new(vertices, vec![0], vec![0], &device, label);
    }
}

pub fn read_obj(path: &str, device: &wgpu::Device) -> Object {
    let mut vertices = vec![];
    let mut faces = vec![];
    let mut edges = vec![];

    let lines_single = fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = lines_single.split("\n").collect();

    for line in lines.iter() {
        if line.starts_with("v") {
            let vertex: Vec<f32> = line.split_ascii_whitespace().skip(1).map(|e| f32::from_str(e).unwrap()).collect();
            vertices.push(Vertex {position: [vertex[0], vertex[1], vertex[2]], color: [vertex[3], vertex[4], vertex[5]]})
        } else if line.starts_with("f") {
            let face: Vec<u16> = line.split_ascii_whitespace().skip(1).map(|e| u16::from_str(e).unwrap()).collect();
            faces.push(face[0]-1);
            faces.push(face[1]-1);
            faces.push(face[2]-1);
        }
    }

    // Although not seemingly directly related, this only somewhat overcounts
    // each triangle has 3 edges, but also 3 vertices that make it up
    // so the number of faces is the number of edges (with duplicates), but an exact count would
    // depend on the exact geometry of the object
    let mut edges_set: HashSet<(u16, u16)> = HashSet::with_capacity(faces.len());
    for i in 0..(faces.len() / 3) {
        if !edges_set.contains(&(faces[i*3], faces[i*3+1])) && !edges_set.contains(&(faces[i*3+1], faces[i*3])){
            edges_set.insert((faces[i*3], faces[i*3+1]));
        }
        if !edges_set.contains(&(faces[i*3+1], faces[i*3+2])) && !edges_set.contains(&(faces[i*3+2], faces[i*3+1])){
            edges_set.insert((faces[i*3+1], faces[i*3+2]));
        }
        if !edges_set.contains(&(faces[i*3], faces[i*3+2])) && !edges_set.contains(&(faces[i*3+2], faces[i*3])){
            edges_set.insert((faces[i*3], faces[i*3+2]));
        }
    }

    edges_set.iter().for_each(|e| {edges.push(e.0); edges.push(e.1)});

    return Object::new(vertices, faces, edges, device, Some(path));
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Camera {
    pub position: cgmath::Point3<f32>,
    pub projection: Projection,
    pub yaw: cgmath::Rad<f32>,
    pub pitch: cgmath::Rad<f32>,
    pub rotating: bool,
}

impl Camera {
    pub fn new<V: Into<cgmath::Point3<f32>>, Y: Into<cgmath::Rad<f32>>, P: Into<cgmath::Rad<f32>>, >(position: V, yaw: Y, pitch: P, width: u32, height: u32) -> Self {
        Self {
            position: position.into(),
            projection: Projection::new(width, height, cgmath::Deg(45.0), 0.1, 100.0),
            yaw: yaw.into(),
            pitch: pitch.into(),
            rotating: false
        }
    }

    pub fn calc_matrix_4(&self) -> cgmath::Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        return cgmath::Matrix4::look_to_rh(
            self.position,
            cgmath::Vector3::new(
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw
            ).normalize(),
            cgmath::Vector3::unit_y(),
        );
    }

    pub fn calc_matrix_3(&self) -> cgmath::Matrix3<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
        return cgmath::Matrix3::look_to_rh(
            cgmath::Vector3::new(
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw
            ).normalize(),
            cgmath::Vector3::unit_y(),
        );
    }

    pub fn update_camera(&mut self, code: KeyCode, is_pressed: bool) {
        let (yaw_sin, yaw_cos) = self.yaw.0.sin_cos();
        let forward = cgmath::Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = cgmath::Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        match (code, is_pressed) {
            (KeyCode::KeyW, true) => self.position += forward * 0.05,
            (KeyCode::KeyS, true) => self.position += -forward * 0.05,
            (KeyCode::KeyA, true) => self.position += -right * 0.05,
            (KeyCode::KeyD, true) => self.position += right * 0.05,
            (KeyCode::Space, true) => self.position.y += 0.05,
            (KeyCode::KeyC, true) => self.position.y -= 0.05,
            (_, _) => {}
        }

    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        return self.projection.calc_matrix() * self.calc_matrix_4();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        return Self {view_proj: camera.build_view_projection_matrix().into()};
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

pub struct Projection {
    aspect: f32,
    fovy: cgmath::Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<cgmath::Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> cgmath::Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}