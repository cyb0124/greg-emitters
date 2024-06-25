use super::geometry::{mul_i, mul_ni};
use crate::{jvm::*, mapping_base::MBOptExt, objs};
use alloc::vec::Vec;
use core::ops::AddAssign;
use nalgebra::{point, Point2, Vector2, Vector4};

// Extracted from epaint of egui

pub struct Stroke {
    pub width: f32,
    pub color: Vector4<f32>,
}

impl Stroke {
    pub fn new(width: f32, color: Vector4<f32>) -> Self { Self { width, color } }
}

pub struct Vertex {
    pub pos: Point2<f32>,
    pub color: Vector4<f32>,
}

#[derive(Default)]
pub struct Mesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn reserve_triangles(&mut self, additional_triangles: usize) { self.indices.reserve(3 * additional_triangles) }
    pub fn reserve_vertices(&mut self, additional: usize) { self.vertices.reserve(additional) }
    pub fn colored_vertex(&mut self, pos: Point2<f32>, color: Vector4<f32>) { self.vertices.push(Vertex { pos, color }); }
    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }
}

#[derive(PartialEq)]
pub struct Rounding {
    pub nw: f32,
    pub ne: f32,
    pub sw: f32,
    pub se: f32,
}

impl Rounding {
    pub const ZERO: Self = Self { nw: 0.0, ne: 0.0, sw: 0.0, se: 0.0 };
    pub const fn same(radius: f32) -> Self { Self { nw: radius, ne: radius, sw: radius, se: radius } }
    pub fn at_least(&self, min: f32) -> Self { Self { nw: self.nw.max(min), ne: self.ne.max(min), sw: self.sw.max(min), se: self.se.max(min) } }
    pub fn at_most(&self, max: f32) -> Self { Self { nw: self.nw.min(max), ne: self.ne.min(max), sw: self.sw.min(max), se: self.se.min(max) } }
}

impl AddAssign for Rounding {
    fn add_assign(&mut self, rhs: Self) { *self = Self { nw: self.nw + rhs.nw, ne: self.ne + rhs.ne, sw: self.sw + rhs.sw, se: self.se + rhs.se }; }
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub min: Point2<f32>,
    pub max: Point2<f32>,
}

impl Rect {
    pub fn width(&self) -> f32 { self.max.x - self.min.x }
    pub fn height(&self) -> f32 { self.max.y - self.min.y }
    pub fn size(&self) -> Vector2<f32> { self.max - self.min }
    pub fn center(&self) -> Point2<f32> { self.min.lerp(&self.max, 0.5) }
    pub fn center_top(&self) -> Point2<f32> { point!(self.center().x, self.min.y) }
    pub fn center_bottom(&self) -> Point2<f32> { point!(self.center().x, self.max.y) }
    pub fn left_center(&self) -> Point2<f32> { point!(self.min.x, self.center().y) }
    pub fn right_center(&self) -> Point2<f32> { point!(self.max.x, self.center().y) }
}

mod precomputed_vertices {
    use nalgebra::{vector, Vector2};

    pub const CIRCLE_8: [Vector2<f32>; 9] = [
        vector!(1.000000, 0.000000),
        vector!(0.707107, 0.707107),
        vector!(0.000000, 1.000000),
        vector!(-0.707107, 0.707107),
        vector!(-1.000000, 0.000000),
        vector!(-0.707107, -0.707107),
        vector!(0.000000, -1.000000),
        vector!(0.707107, -0.707107),
        vector!(1.000000, 0.000000),
    ];

    pub const CIRCLE_16: [Vector2<f32>; 17] = [
        vector!(1.000000, 0.000000),
        vector!(0.923880, 0.382683),
        vector!(0.707107, 0.707107),
        vector!(0.382683, 0.923880),
        vector!(0.000000, 1.000000),
        vector!(-0.382684, 0.923880),
        vector!(-0.707107, 0.707107),
        vector!(-0.923880, 0.382683),
        vector!(-1.000000, 0.000000),
        vector!(-0.923880, -0.382683),
        vector!(-0.707107, -0.707107),
        vector!(-0.382684, -0.923880),
        vector!(0.000000, -1.000000),
        vector!(0.382684, -0.923879),
        vector!(0.707107, -0.707107),
        vector!(0.923880, -0.382683),
        vector!(1.000000, 0.000000),
    ];

    pub const CIRCLE_32: [Vector2<f32>; 33] = [
        vector!(1.000000, 0.000000),
        vector!(0.980785, 0.195090),
        vector!(0.923880, 0.382683),
        vector!(0.831470, 0.555570),
        vector!(0.707107, 0.707107),
        vector!(0.555570, 0.831470),
        vector!(0.382683, 0.923880),
        vector!(0.195090, 0.980785),
        vector!(0.000000, 1.000000),
        vector!(-0.195090, 0.980785),
        vector!(-0.382683, 0.923880),
        vector!(-0.555570, 0.831470),
        vector!(-0.707107, 0.707107),
        vector!(-0.831470, 0.555570),
        vector!(-0.923880, 0.382683),
        vector!(-0.980785, 0.195090),
        vector!(-1.000000, 0.000000),
        vector!(-0.980785, -0.195090),
        vector!(-0.923880, -0.382683),
        vector!(-0.831470, -0.555570),
        vector!(-0.707107, -0.707107),
        vector!(-0.555570, -0.831470),
        vector!(-0.382683, -0.923880),
        vector!(-0.195090, -0.980785),
        vector!(-0.000000, -1.000000),
        vector!(0.195090, -0.980785),
        vector!(0.382683, -0.923880),
        vector!(0.555570, -0.831470),
        vector!(0.707107, -0.707107),
        vector!(0.831470, -0.555570),
        vector!(0.923880, -0.382683),
        vector!(0.980785, -0.195090),
        vector!(1.000000, -0.000000),
    ];

    pub const CIRCLE_64: [Vector2<f32>; 65] = [
        vector!(1.000000, 0.000000),
        vector!(0.995185, 0.098017),
        vector!(0.980785, 0.195090),
        vector!(0.956940, 0.290285),
        vector!(0.923880, 0.382683),
        vector!(0.881921, 0.471397),
        vector!(0.831470, 0.555570),
        vector!(0.773010, 0.634393),
        vector!(0.707107, 0.707107),
        vector!(0.634393, 0.773010),
        vector!(0.555570, 0.831470),
        vector!(0.471397, 0.881921),
        vector!(0.382683, 0.923880),
        vector!(0.290285, 0.956940),
        vector!(0.195090, 0.980785),
        vector!(0.098017, 0.995185),
        vector!(0.000000, 1.000000),
        vector!(-0.098017, 0.995185),
        vector!(-0.195090, 0.980785),
        vector!(-0.290285, 0.956940),
        vector!(-0.382683, 0.923880),
        vector!(-0.471397, 0.881921),
        vector!(-0.555570, 0.831470),
        vector!(-0.634393, 0.773010),
        vector!(-0.707107, 0.707107),
        vector!(-0.773010, 0.634393),
        vector!(-0.831470, 0.555570),
        vector!(-0.881921, 0.471397),
        vector!(-0.923880, 0.382683),
        vector!(-0.956940, 0.290285),
        vector!(-0.980785, 0.195090),
        vector!(-0.995185, 0.098017),
        vector!(-1.000000, 0.000000),
        vector!(-0.995185, -0.098017),
        vector!(-0.980785, -0.195090),
        vector!(-0.956940, -0.290285),
        vector!(-0.923880, -0.382683),
        vector!(-0.881921, -0.471397),
        vector!(-0.831470, -0.555570),
        vector!(-0.773010, -0.634393),
        vector!(-0.707107, -0.707107),
        vector!(-0.634393, -0.773010),
        vector!(-0.555570, -0.831470),
        vector!(-0.471397, -0.881921),
        vector!(-0.382683, -0.923880),
        vector!(-0.290285, -0.956940),
        vector!(-0.195090, -0.980785),
        vector!(-0.098017, -0.995185),
        vector!(-0.000000, -1.000000),
        vector!(0.098017, -0.995185),
        vector!(0.195090, -0.980785),
        vector!(0.290285, -0.956940),
        vector!(0.382683, -0.923880),
        vector!(0.471397, -0.881921),
        vector!(0.555570, -0.831470),
        vector!(0.634393, -0.773010),
        vector!(0.707107, -0.707107),
        vector!(0.773010, -0.634393),
        vector!(0.831470, -0.555570),
        vector!(0.881921, -0.471397),
        vector!(0.923880, -0.382683),
        vector!(0.956940, -0.290285),
        vector!(0.980785, -0.195090),
        vector!(0.995185, -0.098017),
        vector!(1.000000, -0.000000),
    ];

    pub const CIRCLE_128: [Vector2<f32>; 129] = [
        vector!(1.000000, 0.000000),
        vector!(0.998795, 0.049068),
        vector!(0.995185, 0.098017),
        vector!(0.989177, 0.146730),
        vector!(0.980785, 0.195090),
        vector!(0.970031, 0.242980),
        vector!(0.956940, 0.290285),
        vector!(0.941544, 0.336890),
        vector!(0.923880, 0.382683),
        vector!(0.903989, 0.427555),
        vector!(0.881921, 0.471397),
        vector!(0.857729, 0.514103),
        vector!(0.831470, 0.555570),
        vector!(0.803208, 0.595699),
        vector!(0.773010, 0.634393),
        vector!(0.740951, 0.671559),
        vector!(0.707107, 0.707107),
        vector!(0.671559, 0.740951),
        vector!(0.634393, 0.773010),
        vector!(0.595699, 0.803208),
        vector!(0.555570, 0.831470),
        vector!(0.514103, 0.857729),
        vector!(0.471397, 0.881921),
        vector!(0.427555, 0.903989),
        vector!(0.382683, 0.923880),
        vector!(0.336890, 0.941544),
        vector!(0.290285, 0.956940),
        vector!(0.242980, 0.970031),
        vector!(0.195090, 0.980785),
        vector!(0.146730, 0.989177),
        vector!(0.098017, 0.995185),
        vector!(0.049068, 0.998795),
        vector!(0.000000, 1.000000),
        vector!(-0.049068, 0.998795),
        vector!(-0.098017, 0.995185),
        vector!(-0.146730, 0.989177),
        vector!(-0.195090, 0.980785),
        vector!(-0.242980, 0.970031),
        vector!(-0.290285, 0.956940),
        vector!(-0.336890, 0.941544),
        vector!(-0.382683, 0.923880),
        vector!(-0.427555, 0.903989),
        vector!(-0.471397, 0.881921),
        vector!(-0.514103, 0.857729),
        vector!(-0.555570, 0.831470),
        vector!(-0.595699, 0.803208),
        vector!(-0.634393, 0.773010),
        vector!(-0.671559, 0.740951),
        vector!(-0.707107, 0.707107),
        vector!(-0.740951, 0.671559),
        vector!(-0.773010, 0.634393),
        vector!(-0.803208, 0.595699),
        vector!(-0.831470, 0.555570),
        vector!(-0.857729, 0.514103),
        vector!(-0.881921, 0.471397),
        vector!(-0.903989, 0.427555),
        vector!(-0.923880, 0.382683),
        vector!(-0.941544, 0.336890),
        vector!(-0.956940, 0.290285),
        vector!(-0.970031, 0.242980),
        vector!(-0.980785, 0.195090),
        vector!(-0.989177, 0.146730),
        vector!(-0.995185, 0.098017),
        vector!(-0.998795, 0.049068),
        vector!(-1.000000, 0.000000),
        vector!(-0.998795, -0.049068),
        vector!(-0.995185, -0.098017),
        vector!(-0.989177, -0.146730),
        vector!(-0.980785, -0.195090),
        vector!(-0.970031, -0.242980),
        vector!(-0.956940, -0.290285),
        vector!(-0.941544, -0.336890),
        vector!(-0.923880, -0.382683),
        vector!(-0.903989, -0.427555),
        vector!(-0.881921, -0.471397),
        vector!(-0.857729, -0.514103),
        vector!(-0.831470, -0.555570),
        vector!(-0.803208, -0.595699),
        vector!(-0.773010, -0.634393),
        vector!(-0.740951, -0.671559),
        vector!(-0.707107, -0.707107),
        vector!(-0.671559, -0.740951),
        vector!(-0.634393, -0.773010),
        vector!(-0.595699, -0.803208),
        vector!(-0.555570, -0.831470),
        vector!(-0.514103, -0.857729),
        vector!(-0.471397, -0.881921),
        vector!(-0.427555, -0.903989),
        vector!(-0.382683, -0.923880),
        vector!(-0.336890, -0.941544),
        vector!(-0.290285, -0.956940),
        vector!(-0.242980, -0.970031),
        vector!(-0.195090, -0.980785),
        vector!(-0.146730, -0.989177),
        vector!(-0.098017, -0.995185),
        vector!(-0.049068, -0.998795),
        vector!(-0.000000, -1.000000),
        vector!(0.049068, -0.998795),
        vector!(0.098017, -0.995185),
        vector!(0.146730, -0.989177),
        vector!(0.195090, -0.980785),
        vector!(0.242980, -0.970031),
        vector!(0.290285, -0.956940),
        vector!(0.336890, -0.941544),
        vector!(0.382683, -0.923880),
        vector!(0.427555, -0.903989),
        vector!(0.471397, -0.881921),
        vector!(0.514103, -0.857729),
        vector!(0.555570, -0.831470),
        vector!(0.595699, -0.803208),
        vector!(0.634393, -0.773010),
        vector!(0.671559, -0.740951),
        vector!(0.707107, -0.707107),
        vector!(0.740951, -0.671559),
        vector!(0.773010, -0.634393),
        vector!(0.803208, -0.595699),
        vector!(0.831470, -0.555570),
        vector!(0.857729, -0.514103),
        vector!(0.881921, -0.471397),
        vector!(0.903989, -0.427555),
        vector!(0.923880, -0.382683),
        vector!(0.941544, -0.336890),
        vector!(0.956940, -0.290285),
        vector!(0.970031, -0.242980),
        vector!(0.980785, -0.195090),
        vector!(0.989177, -0.146730),
        vector!(0.995185, -0.098017),
        vector!(0.998795, -0.049068),
        vector!(1.000000, -0.000000),
    ];
}

#[derive(Clone, Default)]
struct PathPoint {
    pos: Point2<f32>,
    normal: Vector2<f32>,
}

#[derive(Clone, Default)]
pub struct Path(Vec<PathPoint>);

impl Path {
    pub fn clear(&mut self) { self.0.clear(); }
    pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional); }
    pub fn add_point(&mut self, pos: Point2<f32>, normal: Vector2<f32>) { self.0.push(PathPoint { pos, normal }); }
    pub fn add_circle(&mut self, center: Point2<f32>, radius: f32) {
        use precomputed_vertices::*;
        if radius <= 2.0 {
            self.0.extend(CIRCLE_8.iter().map(|&n| PathPoint { pos: center + radius * n, normal: n }));
        } else if radius <= 5.0 {
            self.0.extend(CIRCLE_16.iter().map(|&n| PathPoint { pos: center + radius * n, normal: n }));
        } else if radius < 18.0 {
            self.0.extend(CIRCLE_32.iter().map(|&n| PathPoint { pos: center + radius * n, normal: n }));
        } else if radius < 50.0 {
            self.0.extend(CIRCLE_64.iter().map(|&n| PathPoint { pos: center + radius * n, normal: n }));
        } else {
            self.0.extend(CIRCLE_128.iter().map(|&n| PathPoint { pos: center + radius * n, normal: n }));
        }
    }

    pub fn add_line_segment(&mut self, points: [Point2<f32>; 2]) {
        self.reserve(2);
        let normal = mul_ni((points[1] - points[0]).normalize());
        self.add_point(points[0], normal);
        self.add_point(points[1], normal);
    }

    pub fn add_open_points(&mut self, points: &[Point2<f32>]) {
        let n = points.len();
        if n == 2 {
            self.add_line_segment([points[0], points[1]]);
        } else {
            self.reserve(n);
            self.add_point(points[0], mul_ni((points[1] - points[0]).normalize()));
            let mut n0 = mul_ni((points[1] - points[0]).normalize());
            for i in 1..n - 1 {
                let mut n1 = mul_ni((points[i + 1] - points[i]).normalize());
                if n0 == Vector2::zeros() {
                    n0 = n1;
                } else if n1 == Vector2::zeros() {
                    n1 = n0;
                }
                let normal = (n0 + n1) / 2.0;
                let length_sq = normal.norm_squared();
                let right_angle_length_sq = 0.5;
                let sharper_than_a_right_angle = length_sq < right_angle_length_sq;
                if sharper_than_a_right_angle {
                    let center_normal = normal.normalize();
                    let n0c = (n0 + center_normal) / 2.0;
                    let n1c = (n1 + center_normal) / 2.0;
                    self.add_point(points[i], n0c / n0c.norm_squared());
                    self.add_point(points[i], n1c / n1c.norm_squared());
                } else {
                    self.add_point(points[i], normal / length_sq);
                }
                n0 = n1;
            }
            self.add_point(points[n - 1], mul_ni((points[n - 1] - points[n - 2]).normalize()));
        }
    }

    pub fn add_line_loop(&mut self, points: &[Point2<f32>]) {
        let n = points.len();
        self.reserve(n);
        let mut n0 = mul_ni((points[0] - points[n - 1]).normalize());
        for i in 0..n {
            let next_i = if i + 1 == n { 0 } else { i + 1 };
            let mut n1 = mul_ni((points[next_i] - points[i]).normalize());
            if n0 == Vector2::zeros() {
                n0 = n1;
            } else if n1 == Vector2::zeros() {
                n1 = n0;
            }
            let normal = (n0 + n1) / 2.0;
            let length_sq = normal.norm_squared();
            self.add_point(points[i], normal / length_sq);
            n0 = n1;
        }
    }

    pub fn stroke(&self, feathering: f32, closed: bool, stroke: &Stroke, out: &mut Mesh) { stroke_path(feathering, &self.0, closed, stroke, out) }
    pub fn fill(&mut self, feathering: f32, color: Vector4<f32>, out: &mut Mesh) { fill_closed_path(feathering, &mut self.0, color, out) }
}

pub mod path {
    use super::{Rect, Rounding};
    use alloc::vec::Vec;
    use nalgebra::{point, Point2};

    pub fn rounded_rectangle(path: &mut Vec<Point2<f32>>, rect: Rect, rounding: Rounding) {
        path.clear();
        let min = rect.min;
        let max = rect.max;
        let r = clamp_rounding(rounding, rect);
        if r == Rounding::ZERO {
            path.reserve(4);
            path.push(point!(min.x, min.y));
            path.push(point!(max.x, min.y));
            path.push(point!(max.x, max.y));
            path.push(point!(min.x, max.y));
        } else {
            let eps = f32::EPSILON * rect.size().max();
            add_circle_quadrant(path, point!(max.x - r.se, max.y - r.se), r.se, 0.0);
            if rect.width() <= r.se + r.sw + eps {
                path.pop();
            }
            add_circle_quadrant(path, point!(min.x + r.sw, max.y - r.sw), r.sw, 1.0);
            if rect.height() <= r.sw + r.nw + eps {
                path.pop();
            }
            add_circle_quadrant(path, point!(min.x + r.nw, min.y + r.nw), r.nw, 2.0);
            if rect.width() <= r.nw + r.ne + eps {
                path.pop();
            }
            add_circle_quadrant(path, point!(max.x - r.ne, min.y + r.ne), r.ne, 3.0);
            if rect.height() <= r.ne + r.se + eps {
                path.pop();
            }
        }
    }

    pub fn add_circle_quadrant(path: &mut Vec<Point2<f32>>, center: Point2<f32>, radius: f32, quadrant: f32) {
        use super::precomputed_vertices::*;
        if radius <= 0.0 {
            path.push(center);
        } else if radius <= 2.0 {
            let offset = quadrant as usize * 2;
            let quadrant_vertices = &CIRCLE_8[offset..=offset + 2];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else if radius <= 5.0 {
            let offset = quadrant as usize * 4;
            let quadrant_vertices = &CIRCLE_16[offset..=offset + 4];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else if radius < 18.0 {
            let offset = quadrant as usize * 8;
            let quadrant_vertices = &CIRCLE_32[offset..=offset + 8];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else if radius < 50.0 {
            let offset = quadrant as usize * 16;
            let quadrant_vertices = &CIRCLE_64[offset..=offset + 16];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        } else {
            let offset = quadrant as usize * 32;
            let quadrant_vertices = &CIRCLE_128[offset..=offset + 32];
            path.extend(quadrant_vertices.iter().map(|&n| center + radius * n));
        }
    }

    fn clamp_rounding(rounding: Rounding, rect: Rect) -> Rounding {
        let half_width = rect.width() * 0.5;
        let half_height = rect.height() * 0.5;
        let max_cr = half_width.min(half_height);
        rounding.at_most(max_cr).at_least(0.0)
    }
}

fn cw_signed_area(path: &[PathPoint]) -> f64 {
    if let Some(last) = path.last() {
        let mut previous = last.pos;
        let mut area = 0.0;
        for p in path {
            area += (previous.x * p.pos.y - p.pos.x * previous.y) as f64;
            previous = p.pos;
        }
        area
    } else {
        0.0
    }
}

fn fill_closed_path(feathering: f32, path: &mut [PathPoint], color: Vector4<f32>, out: &mut Mesh) {
    if color.w == 0. {
        return;
    }
    let n = path.len() as u32;
    if cw_signed_area(path) < 0.0 {
        path.reverse();
        for point in &mut *path {
            point.normal = -point.normal;
        }
    }
    out.reserve_triangles(3 * n as usize);
    out.reserve_vertices(2 * n as usize);
    let color_outer = mul_color(color, 0.);
    let idx_inner = out.vertices.len() as u32;
    let idx_outer = idx_inner + 1;
    for i in 2..n {
        out.add_triangle(idx_inner + 2 * (i - 1), idx_inner, idx_inner + 2 * i);
    }
    let mut i0 = n - 1;
    for i1 in 0..n {
        let p1 = &path[i1 as usize];
        let dm = 0.5 * feathering * p1.normal;
        out.colored_vertex(p1.pos - dm, color);
        out.colored_vertex(p1.pos + dm, color_outer);
        out.add_triangle(idx_inner + i1 * 2, idx_inner + i0 * 2, idx_outer + 2 * i0);
        out.add_triangle(idx_outer + i0 * 2, idx_outer + i1 * 2, idx_inner + 2 * i1);
        i0 = i1;
    }
}

fn stroke_path(feathering: f32, path: &[PathPoint], closed: bool, stroke: &Stroke, out: &mut Mesh) {
    let n = path.len() as u32;
    if stroke.width <= 0.0 || stroke.color.w == 0. || n < 2 {
        return;
    }
    let idx = out.vertices.len() as u32;
    let color_inner = stroke.color;
    let color_outer = mul_color(color_inner, 0.);
    if stroke.width <= feathering {
        let color_inner = mul_color(color_inner, stroke.width / feathering);
        if color_inner.w == 0. {
            return;
        }
        out.reserve_triangles(4 * n as usize);
        out.reserve_vertices(3 * n as usize);
        let mut i0 = n - 1;
        for i1 in 0..n {
            let connect_with_previous = closed || i1 > 0;
            let p1 = &path[i1 as usize];
            let p = p1.pos;
            let n = p1.normal;
            out.colored_vertex(p + n * feathering, color_outer);
            out.colored_vertex(p, color_inner);
            out.colored_vertex(p - n * feathering, color_outer);
            if connect_with_previous {
                out.add_triangle(idx + 3 * i0 + 0, idx + 3 * i0 + 1, idx + 3 * i1 + 0);
                out.add_triangle(idx + 3 * i0 + 1, idx + 3 * i1 + 0, idx + 3 * i1 + 1);
                out.add_triangle(idx + 3 * i0 + 1, idx + 3 * i0 + 2, idx + 3 * i1 + 1);
                out.add_triangle(idx + 3 * i0 + 2, idx + 3 * i1 + 1, idx + 3 * i1 + 2);
            }
            i0 = i1;
        }
    } else {
        let inner_rad = 0.5 * (stroke.width - feathering);
        let outer_rad = 0.5 * (stroke.width + feathering);
        if closed {
            out.reserve_triangles(6 * n as usize);
            out.reserve_vertices(4 * n as usize);
            let mut i0 = n - 1;
            for i1 in 0..n {
                let p1 = &path[i1 as usize];
                let p = p1.pos;
                let n = p1.normal;
                out.colored_vertex(p + n * outer_rad, color_outer);
                out.colored_vertex(p + n * inner_rad, color_inner);
                out.colored_vertex(p - n * inner_rad, color_inner);
                out.colored_vertex(p - n * outer_rad, color_outer);
                out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);
                out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                i0 = i1;
            }
        } else {
            out.reserve_triangles(6 * n as usize + 4);
            out.reserve_vertices(4 * n as usize);
            {
                let end = &path[0];
                let p = end.pos;
                let n = end.normal;
                let back_extrude = mul_ni(n) * feathering;
                out.colored_vertex(p + n * outer_rad + back_extrude, color_outer);
                out.colored_vertex(p + n * inner_rad, color_inner);
                out.colored_vertex(p - n * inner_rad, color_inner);
                out.colored_vertex(p - n * outer_rad + back_extrude, color_outer);
                out.add_triangle(idx + 0, idx + 1, idx + 2);
                out.add_triangle(idx + 0, idx + 2, idx + 3);
            }
            let mut i0 = 0;
            for i1 in 1..n - 1 {
                let point = &path[i1 as usize];
                let p = point.pos;
                let n = point.normal;
                out.colored_vertex(p + n * outer_rad, color_outer);
                out.colored_vertex(p + n * inner_rad, color_inner);
                out.colored_vertex(p - n * inner_rad, color_inner);
                out.colored_vertex(p - n * outer_rad, color_outer);
                out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);
                out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                i0 = i1;
            }
            {
                let i1 = n - 1;
                let end = &path[i1 as usize];
                let p = end.pos;
                let n = end.normal;
                let back_extrude = mul_i(n) * feathering;
                out.colored_vertex(p + n * outer_rad + back_extrude, color_outer);
                out.colored_vertex(p + n * inner_rad, color_inner);
                out.colored_vertex(p - n * inner_rad, color_inner);
                out.colored_vertex(p - n * outer_rad + back_extrude, color_outer);
                out.add_triangle(idx + 4 * i0 + 0, idx + 4 * i0 + 1, idx + 4 * i1 + 0);
                out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i1 + 0, idx + 4 * i1 + 1);
                out.add_triangle(idx + 4 * i0 + 1, idx + 4 * i0 + 2, idx + 4 * i1 + 1);
                out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i1 + 1, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i0 + 2, idx + 4 * i0 + 3, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i0 + 3, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
                out.add_triangle(idx + 4 * i1 + 0, idx + 4 * i1 + 1, idx + 4 * i1 + 2);
                out.add_triangle(idx + 4 * i1 + 0, idx + 4 * i1 + 2, idx + 4 * i1 + 3);
            }
        }
    }
}

fn mul_color(mut color: Vector4<f32>, factor: f32) -> Vector4<f32> {
    color.w *= factor;
    color
}

pub struct Tessellator {
    feathering: f32,
    tmp_path: Path,
    tmp_pts: Vec<Point2<f32>>,
    pub mesh: Mesh,
}

impl Tessellator {
    pub fn new(jni: &JNI) -> Self {
        let mvc = objs().mv.client.uref();
        let gui_scale = mvc.window_inst.with_jni(jni).call_double_method(mvc.window_get_gui_scale, &[]).unwrap();
        Self { feathering: gui_scale.recip() as _, tmp_path: Path::default(), tmp_pts: Vec::new(), mesh: Mesh::default() }
    }

    pub fn circle(&mut self, center: Point2<f32>, radius: f32, fill: Vector4<f32>, stroke: &Stroke) {
        if radius <= 0.0 {
            return;
        }
        self.tmp_path.clear();
        self.tmp_path.add_circle(center, radius);
        self.tmp_path.fill(self.feathering, fill, &mut self.mesh);
        self.tmp_path.stroke(self.feathering, true, stroke, &mut self.mesh)
    }

    pub fn line(&mut self, points: [Point2<f32>; 2], stroke: &Stroke) {
        self.tmp_path.clear();
        self.tmp_path.add_line_segment(points);
        self.tmp_path.stroke(self.feathering, false, stroke, &mut self.mesh)
    }

    pub fn path(&mut self, points: &[Point2<f32>], closed: bool, fill: Vector4<f32>, stroke: &Stroke) {
        if points.len() < 2 {
            return;
        }
        self.tmp_path.clear();
        if closed {
            self.tmp_path.add_line_loop(points)
        } else {
            self.tmp_path.add_open_points(points)
        }
        self.tmp_path.fill(self.feathering, fill, &mut self.mesh);
        self.tmp_path.stroke(self.feathering, closed, stroke, &mut self.mesh)
    }

    pub fn rect(&mut self, rect: Rect, mut rounding: Rounding, mut blur_width: f32, fill: Vector4<f32>, stroke: &Stroke) {
        let old_feathering = self.feathering;
        if self.feathering < blur_width {
            blur_width = blur_width.min(rect.size().min() - 0.1).max(0.);
            rounding += Rounding::same(0.5 * blur_width);
            self.feathering = self.feathering.max(blur_width)
        }
        if rect.width() < self.feathering {
            let line = [rect.center_top(), rect.center_bottom()];
            self.line(line, &Stroke::new(rect.width(), fill));
            self.line(line, stroke);
            self.line(line, stroke)
        } else if rect.height() < self.feathering {
            let line = [rect.left_center(), rect.right_center()];
            self.line(line, &Stroke::new(rect.height(), fill));
            self.line(line, stroke);
            self.line(line, stroke);
        } else {
            self.tmp_path.clear();
            path::rounded_rectangle(&mut self.tmp_pts, rect, rounding);
            self.tmp_path.add_line_loop(&self.tmp_pts);
            self.tmp_path.fill(self.feathering, fill, &mut self.mesh);
            self.tmp_path.stroke(self.feathering, true, stroke, &mut self.mesh);
        }
        self.feathering = old_feathering
    }
}
