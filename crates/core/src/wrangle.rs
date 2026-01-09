use std::collections::HashMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage, AttributeType};
use crate::mesh::Mesh;
use crate::nodes::recompute_mesh_normals;
use crate::splat::SplatGeo;
use crate::volume::Volume;
use crate::volume_sampling::VolumeSampler;

#[derive(Debug, Clone)]
struct Program {
    statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
enum Statement {
    Assign { target: String, expr: Expr },
}

#[derive(Debug, Clone)]
enum Expr {
    Literal(Value),
    Attr(String),
    Ident(String),
    Swizzle {
        expr: Box<Expr>,
        mask: String,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
enum UnaryOp {
    Pos,
    Neg,
}

#[derive(Debug, Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy)]
enum Value {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

impl Value {
    fn data_type(self) -> AttributeType {
        match self {
            Value::Float(_) => AttributeType::Float,
            Value::Vec2(_) => AttributeType::Vec2,
            Value::Vec3(_) => AttributeType::Vec3,
            Value::Vec4(_) => AttributeType::Vec4,
        }
    }

    fn negate(self) -> Value {
        map_value(self, |v| -v)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Number(f32),
    At,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
    Equal,
    Semicolon,
}

#[allow(clippy::too_many_arguments)]
pub fn apply_wrangle(
    mesh: &mut Mesh,
    domain: AttributeDomain,
    code: &str,
    mask: Option<&[bool]>,
    secondary_mesh: Option<&Mesh>,
    primary_splats: Option<&SplatGeo>,
    secondary_splats: Option<&SplatGeo>,
    primary_volume: Option<&Volume>,
    secondary_volume: Option<&Volume>,
) -> Result<(), String> {
    let program = parse_program(code)?;
    if program.statements.is_empty() {
        return Ok(());
    }
    let len = mesh.attribute_domain_len(domain);
    if len == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }
    if let Some(mask) = mask {
        if domain != AttributeDomain::Detail && mask.len() != len {
            return Ok(());
        }
    }

    let mut ctx =
        WrangleContext::new(
            mesh,
            domain,
            mask,
            secondary_mesh,
            primary_splats,
            secondary_splats,
            primary_volume,
            secondary_volume,
        );
    for stmt in program.statements {
        ctx.apply_statement(stmt)?;
    }
    let written = ctx.into_written();
    let wrote_positions = written.contains_key("P") && domain == AttributeDomain::Point;
    apply_written(mesh, domain, written)?;
    if wrote_positions {
        recompute_mesh_normals(mesh);
    }
    Ok(())
}

pub fn apply_wrangle_splats(
    splats: &mut SplatGeo,
    domain: AttributeDomain,
    code: &str,
    mask: Option<&[bool]>,
    secondary: Option<&SplatGeo>,
    primary_volume: Option<&Volume>,
    secondary_volume: Option<&Volume>,
) -> Result<(), String> {
    let program = parse_program(code)?;
    if program.statements.is_empty() {
        return Ok(());
    }
    let len = splats.attribute_domain_len(domain);
    if len == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }
    if let Some(mask) = mask {
        if domain != AttributeDomain::Detail && mask.len() != len {
            return Ok(());
        }
    }

    let mut ctx = SplatWrangleContext::new(
        splats,
        domain,
        mask,
        secondary,
        primary_volume,
        secondary_volume,
    );
    for stmt in program.statements {
        ctx.apply_statement(stmt)?;
    }
    let written = ctx.into_written();
    apply_written_splats(splats, domain, written)?;
    Ok(())
}

struct MeshQueryCache<'a> {
    mesh: &'a Mesh,
    point_normals: Option<Vec<[f32; 3]>>,
    vertex_normals: Option<Vec<[f32; 3]>>,
    prim_normals: Option<Vec<[f32; 3]>>,
    prim_centers: Option<Vec<[f32; 3]>>,
    detail_center: Option<[f32; 3]>,
    detail_normal: Option<[f32; 3]>,
}

impl<'a> MeshQueryCache<'a> {
    fn new(mesh: &'a Mesh) -> Self {
        Self {
            mesh,
            point_normals: None,
            vertex_normals: None,
            prim_normals: None,
            prim_centers: None,
            detail_center: None,
            detail_normal: None,
        }
    }

    fn read_p(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point => self.mesh.positions.get(idx).copied().unwrap_or([0.0; 3]),
            AttributeDomain::Vertex => {
                let point = self.mesh.indices.get(idx).copied().unwrap_or(0) as usize;
                self.mesh.positions.get(point).copied().unwrap_or([0.0; 3])
            }
            AttributeDomain::Primitive => {
                self.ensure_prim_centers();
                self.prim_centers
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0; 3])
            }
            AttributeDomain::Detail => {
                self.ensure_detail_center();
                self.detail_center.unwrap_or([0.0; 3])
            }
        }
    }

    fn read_n(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point => {
                self.ensure_point_normals();
                self.point_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Vertex => {
                self.ensure_vertex_normals();
                self.vertex_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Primitive => {
                self.ensure_prim_normals();
                self.prim_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Detail => {
                self.ensure_detail_normal();
                self.detail_normal.unwrap_or([0.0, 1.0, 0.0])
            }
        }
    }

    fn ensure_point_normals(&mut self) {
        if self.point_normals.is_some() {
            return;
        }
        if let Some(normals) = &self.mesh.normals {
            self.point_normals = Some(normals.clone());
            return;
        }
        self.point_normals = Some(compute_point_normals(self.mesh));
    }

    fn ensure_vertex_normals(&mut self) {
        if self.vertex_normals.is_some() {
            return;
        }
        if let Some(normals) = &self.mesh.corner_normals {
            self.vertex_normals = Some(normals.clone());
            return;
        }
        self.ensure_point_normals();
        let point_normals = self.point_normals.as_ref().cloned().unwrap_or_default();
        let mut result = Vec::with_capacity(self.mesh.indices.len());
        for idx in &self.mesh.indices {
            let point = *idx as usize;
            result.push(point_normals.get(point).copied().unwrap_or([0.0, 1.0, 0.0]));
        }
        self.vertex_normals = Some(result);
    }

    fn ensure_prim_normals(&mut self) {
        if self.prim_normals.is_some() {
            return;
        }
        let mut normals = Vec::new();
        for tri in self.mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            let p0 = Vec3::from(self.mesh.positions.get(i0).copied().unwrap_or([0.0; 3]));
            let p1 = Vec3::from(self.mesh.positions.get(i1).copied().unwrap_or([0.0; 3]));
            let p2 = Vec3::from(self.mesh.positions.get(i2).copied().unwrap_or([0.0; 3]));
            let n = (p1 - p0).cross(p2 - p0);
            let n = if n.length_squared() > 0.0 {
                n.normalize().to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
            normals.push(n);
        }
        self.prim_normals = Some(normals);
    }

    fn ensure_prim_centers(&mut self) {
        if self.prim_centers.is_some() {
            return;
        }
        let mut centers = Vec::new();
        for tri in self.mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            let p0 = Vec3::from(self.mesh.positions.get(i0).copied().unwrap_or([0.0; 3]));
            let p1 = Vec3::from(self.mesh.positions.get(i1).copied().unwrap_or([0.0; 3]));
            let p2 = Vec3::from(self.mesh.positions.get(i2).copied().unwrap_or([0.0; 3]));
            let center = (p0 + p1 + p2) / 3.0;
            centers.push(center.to_array());
        }
        self.prim_centers = Some(centers);
    }

    fn ensure_detail_center(&mut self) {
        if self.detail_center.is_some() {
            return;
        }
        let center = self.mesh.bounds().map(|bounds| {
            [
                (bounds.min[0] + bounds.max[0]) * 0.5,
                (bounds.min[1] + bounds.max[1]) * 0.5,
                (bounds.min[2] + bounds.max[2]) * 0.5,
            ]
        });
        self.detail_center = Some(center.unwrap_or([0.0; 3]));
    }

    fn ensure_detail_normal(&mut self) {
        if self.detail_normal.is_some() {
            return;
        }
        self.ensure_point_normals();
        let normals = self.point_normals.as_ref().cloned().unwrap_or_default();
        let mut sum = Vec3::ZERO;
        for n in normals {
            sum += Vec3::from(n);
        }
        let normal = if sum.length_squared() > 0.0 {
            sum.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
        self.detail_normal = Some(normal);
    }
}

struct WrangleContext<'a> {
    mesh: &'a Mesh,
    secondary_query: Option<MeshQueryCache<'a>>,
    primary_splats: Option<SplatQueryCache<'a>>,
    secondary_splats: Option<SplatQueryCache<'a>>,
    primary_volume: Option<VolumeSampler<'a>>,
    secondary_volume: Option<VolumeSampler<'a>>,
    domain: AttributeDomain,
    len: usize,
    mask: Option<&'a [bool]>,
    written: HashMap<String, AttributeStorage>,
    point_normals: Option<Vec<[f32; 3]>>,
    vertex_normals: Option<Vec<[f32; 3]>>,
    prim_normals: Option<Vec<[f32; 3]>>,
    prim_centers: Option<Vec<[f32; 3]>>,
    detail_center: Option<[f32; 3]>,
    detail_normal: Option<[f32; 3]>,
    point_first_vertex: Option<Vec<usize>>,
    point_first_prim: Option<Vec<usize>>,
}

impl<'a> WrangleContext<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        mesh: &'a Mesh,
        domain: AttributeDomain,
        mask: Option<&'a [bool]>,
        secondary_mesh: Option<&'a Mesh>,
        primary_splats: Option<&'a SplatGeo>,
        secondary_splats: Option<&'a SplatGeo>,
        primary_volume: Option<&'a Volume>,
        secondary_volume: Option<&'a Volume>,
    ) -> Self {
        let len = mesh.attribute_domain_len(domain);
        Self {
            mesh,
            secondary_query: secondary_mesh.map(MeshQueryCache::new),
            primary_splats: primary_splats.map(SplatQueryCache::new),
            secondary_splats: secondary_splats.map(SplatQueryCache::new),
            primary_volume: primary_volume.map(VolumeSampler::new),
            secondary_volume: secondary_volume.map(VolumeSampler::new),
            domain,
            len,
            mask,
            written: HashMap::new(),
            point_normals: None,
            vertex_normals: None,
            prim_normals: None,
            prim_centers: None,
            detail_center: None,
            detail_normal: None,
            point_first_vertex: None,
            point_first_prim: None,
        }
    }

    fn apply_statement(&mut self, stmt: Statement) -> Result<(), String> {
        match stmt {
            Statement::Assign { target, expr } => self.assign(target, expr),
        }
    }

    fn assign(&mut self, target: String, expr: Expr) -> Result<(), String> {
        if target == "P" && self.domain != AttributeDomain::Point {
            return Err("Wrangle can only write @P in Point mode".to_string());
        }
        if self.len == 0 {
            return Ok(());
        }

        if self.mask.is_some() && !self.any_selected() {
            return Ok(());
        }

        let target_type = self.target_type(&target).or_else(|| {
            let idx = self.first_selected_index().unwrap_or(0);
            self.eval_expr(&expr, idx).ok().map(|value| value.data_type())
        });

        let mut values = Vec::with_capacity(self.len.max(1));
        for idx in 0..self.len.max(1) {
            let selected = self
                .mask
                .and_then(|mask| mask.get(idx).copied())
                .unwrap_or(true);
            let value = if selected {
                self.eval_expr(&expr, idx)?
            } else {
                self.read_attr_for_mask(&target, idx, target_type)?
            };
            values.push(value);
        }

        let storage = build_storage(&values, target_type)?;
        self.written.insert(target, storage);
        Ok(())
    }

    fn into_written(self) -> HashMap<String, AttributeStorage> {
        self.written
    }

    fn target_type(&self, name: &str) -> Option<AttributeType> {
        if let Some(storage) = self.written.get(name) {
            return Some(storage.data_type());
        }
        match (name, self.domain) {
            ("P", AttributeDomain::Point) => return Some(AttributeType::Vec3),
            ("N", AttributeDomain::Point) => return Some(AttributeType::Vec3),
            ("N", AttributeDomain::Vertex) => return Some(AttributeType::Vec3),
            _ => {}
        }
        self.mesh
            .attribute(self.domain, name)
            .map(|attr| attr.data_type())
    }

    fn eval_expr(&mut self, expr: &Expr, idx: usize) -> Result<Value, String> {
        match expr {
            Expr::Literal(value) => Ok(*value),
            Expr::Attr(name) => self.read_attr(name, idx),
            Expr::Ident(name) => Err(format!("Unknown identifier '{}'", name)),
            Expr::Swizzle { expr, mask } => {
                let value = self.eval_expr(expr, idx)?;
                swizzle_value(value, mask)
            }
            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr, idx)?;
                Ok(match op {
                    UnaryOp::Pos => value,
                    UnaryOp::Neg => value.negate(),
                })
            }
            Expr::Binary { op, left, right } => {
                let a = self.eval_expr(left, idx)?;
                let b = self.eval_expr(right, idx)?;
                match op {
                    BinaryOp::Add => add_values(a, b),
                    BinaryOp::Sub => sub_values(a, b),
                    BinaryOp::Mul => mul_values(a, b),
                    BinaryOp::Div => div_values(a, b),
                }
            }
            Expr::Call { name, args } => self.eval_call(name, args, idx),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr], idx: usize) -> Result<Value, String> {
        let name = name.to_lowercase();
        match name.as_str() {
            "sin" | "cos" | "tan" | "abs" | "floor" | "ceil" => {
                let value = self.eval_args(args, idx, 1)?[0];
                Ok(match name.as_str() {
                    "sin" => map_value(value, f32::sin),
                    "cos" => map_value(value, f32::cos),
                    "tan" => map_value(value, f32::tan),
                    "abs" => map_value(value, f32::abs),
                    "floor" => map_value(value, f32::floor),
                    _ => map_value(value, f32::ceil),
                })
            }
            "pow" => {
                let values = self.eval_args(args, idx, 2)?;
                pow_values(values[0], values[1])
            }
            "min" => {
                let values = self.eval_args(args, idx, 2)?;
                min_values(values[0], values[1])
            }
            "max" => {
                let values = self.eval_args(args, idx, 2)?;
                max_values(values[0], values[1])
            }
            "clamp" => {
                let values = self.eval_args(args, idx, 3)?;
                clamp_values(values[0], values[1], values[2])
            }
            "lerp" => {
                let values = self.eval_args(args, idx, 3)?;
                lerp_values(values[0], values[1], values[2])
            }
            "len" => {
                let value = self.eval_args(args, idx, 1)?[0];
                Ok(Value::Float(length_value(value)))
            }
            "dot" => {
                let values = self.eval_args(args, idx, 2)?;
                let dot = dot_values(values[0], values[1])?;
                Ok(Value::Float(dot))
            }
            "normalize" => {
                let value = self.eval_args(args, idx, 1)?[0];
                normalize_value(value)
            }
            "point" => self.eval_geo_query(AttributeDomain::Point, args, idx),
            "vertex" => self.eval_geo_query(AttributeDomain::Vertex, args, idx),
            "prim" => self.eval_geo_query(AttributeDomain::Primitive, args, idx),
            "splat" => self.eval_splat_query(args, idx),
            "sample" => self.eval_volume_sample(args, idx),
            "vec2" => build_vec(args, idx, 2, self),
            "vec3" => build_vec(args, idx, 3, self),
            "vec4" => build_vec(args, idx, 4, self),
            _ => Err(format!("Unknown function '{}'", name)),
        }
    }

    fn eval_args(
        &mut self,
        args: &[Expr],
        idx: usize,
        expected: usize,
    ) -> Result<Vec<Value>, String> {
        if args.len() != expected {
            return Err(format!(
                "Expected {} argument(s), got {}",
                expected,
                args.len()
            ));
        }
        let mut out = Vec::with_capacity(args.len());
        for arg in args {
            out.push(self.eval_expr(arg, idx)?);
        }
        Ok(out)
    }

    fn eval_geo_query(
        &mut self,
        domain: AttributeDomain,
        args: &[Expr],
        idx: usize,
    ) -> Result<Value, String> {
        if args.len() != 3 {
            return Err(format!("Expected 3 arguments, got {}", args.len()));
        }
        let input_index = value_to_index(self.eval_expr(&args[0], idx)?)?;
        let attr_name = attr_name_arg(&args[1])?;
        let elem_index = value_to_index(self.eval_expr(&args[2], idx)?)?;

        match input_index {
            0 => Ok(self.query_primary_attr(domain, &attr_name, elem_index)),
            1 => Ok(self.query_secondary_attr(domain, &attr_name, elem_index)),
            _ => Err("Input index must be 0 or 1".to_string()),
        }
    }

    fn eval_volume_sample(&mut self, args: &[Expr], idx: usize) -> Result<Value, String> {
        let (input_index, pos_expr) = match args.len() {
            2 => (self.eval_expr(&args[0], idx)?, &args[1]),
            3 => {
                let _ = attr_name_arg(&args[1])?;
                (self.eval_expr(&args[0], idx)?, &args[2])
            }
            _ => {
                return Err(format!(
                    "sample expects 2 or 3 arguments, got {}",
                    args.len()
                ));
            }
        };
        let input_index = value_to_index(input_index)?;
        let pos_value = self.eval_expr(pos_expr, idx)?;
        let pos = value_to_vec3(pos_value)?;

        let sampler = match input_index {
            0 => self.primary_volume.as_ref(),
            1 => self.secondary_volume.as_ref(),
            _ => return Err("Input index must be 0 or 1".to_string()),
        };
        Ok(Value::Float(
            sampler.map(|sampler| sampler.sample_world(pos)).unwrap_or(0.0),
        ))
    }

    fn eval_splat_query(&mut self, args: &[Expr], idx: usize) -> Result<Value, String> {
        if args.len() != 3 {
            return Err(format!("Expected 3 arguments, got {}", args.len()));
        }
        let input_index = value_to_index(self.eval_expr(&args[0], idx)?)?;
        let attr_name = attr_name_arg(&args[1])?;
        let elem_index = value_to_index(self.eval_expr(&args[2], idx)?)?;

        match input_index {
            0 => Ok(self.query_primary_splat_attr(&attr_name, elem_index)),
            1 => Ok(self.query_secondary_splat_attr(&attr_name, elem_index)),
            _ => Err("Input index must be 0 or 1".to_string()),
        }
    }

    fn query_primary_attr(
        &mut self,
        domain: AttributeDomain,
        name: &str,
        idx: usize,
    ) -> Value {
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(self.read_p_for_domain(domain, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(self.read_n_for_domain(domain, idx));
        }
        if let Some(attr) = self.mesh.attribute(domain, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn query_secondary_attr(
        &mut self,
        domain: AttributeDomain,
        name: &str,
        idx: usize,
    ) -> Value {
        let Some(query) = self.secondary_query.as_mut() else {
            return default_query_value(name);
        };
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(query.read_p(domain, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(query.read_n(domain, idx));
        }
        if let Some(attr) = query.mesh.attribute(domain, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn query_primary_splat_attr(&mut self, name: &str, idx: usize) -> Value {
        let Some(query) = self.primary_splats.as_mut() else {
            return default_query_value(name);
        };
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(query.read_p(AttributeDomain::Point, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(query.read_n(AttributeDomain::Point, idx));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Point, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Primitive, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Detail, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn query_secondary_splat_attr(&mut self, name: &str, idx: usize) -> Value {
        let Some(query) = self.secondary_splats.as_mut() else {
            return default_query_value(name);
        };
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(query.read_p(AttributeDomain::Point, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(query.read_n(AttributeDomain::Point, idx));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Point, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Primitive, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Detail, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn read_attr(&mut self, name: &str, idx: usize) -> Result<Value, String> {
        if let Some(storage) = self.written.get(name) {
            return value_from_storage(storage, idx);
        }
        if let Some(value) = self.read_implicit_attr(name, idx) {
            return Ok(value);
        }
        if name == "P" {
            return Ok(Value::Vec3(self.read_p(idx)));
        }
        if name == "N" {
            return Ok(Value::Vec3(self.read_n(idx)));
        }
        if let Some(attr) = self.mesh.attribute(self.domain, name) {
            return value_from_attr_ref(attr, idx);
        }
        Ok(Value::Float(0.0))
    }

    fn read_attr_for_mask(
        &mut self,
        name: &str,
        idx: usize,
        target_type: Option<AttributeType>,
    ) -> Result<Value, String> {
        if let Some(target_type) = target_type {
            if let Some(attr) = self.mesh.attribute(self.domain, name) {
                if attr.data_type() == target_type {
                    return value_from_attr_ref(attr, idx);
                }
            }
            if name == "P" && target_type == AttributeType::Vec3 {
                return Ok(Value::Vec3(self.read_p(idx)));
            }
            if name == "N" && target_type == AttributeType::Vec3 {
                return Ok(Value::Vec3(self.read_n(idx)));
            }
            return Ok(default_value_for_type(target_type));
        }
        self.read_attr(name, idx)
    }

    fn first_selected_index(&self) -> Option<usize> {
        let mask = self.mask?;
        mask.iter().position(|value| *value)
    }

    fn any_selected(&self) -> bool {
        let Some(mask) = self.mask else {
            return true;
        };
        mask.iter().any(|value| *value)
    }

    fn read_implicit_attr(&mut self, name: &str, idx: usize) -> Option<Value> {
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "ptnum" => Some(Value::Float(self.current_ptnum(idx) as f32)),
            "vtxnum" => Some(Value::Float(self.current_vtxnum(idx) as f32)),
            "primnum" => Some(Value::Float(self.current_primnum(idx) as f32)),
            "numpt" => Some(Value::Float(self.mesh.positions.len() as f32)),
            "numvtx" => Some(Value::Float(self.mesh.indices.len() as f32)),
            "numprim" => Some(Value::Float((self.mesh.indices.len() / 3) as f32)),
            _ => None,
        }
    }

    fn current_ptnum(&mut self, idx: usize) -> i32 {
        match self.domain {
            AttributeDomain::Point => idx as i32,
            AttributeDomain::Vertex => self
                .mesh
                .indices
                .get(idx)
                .copied()
                .map(|value| value as i32)
                .unwrap_or(-1),
            AttributeDomain::Primitive => {
                let base = idx * 3;
                self.mesh
                    .indices
                    .get(base)
                    .copied()
                    .map(|value| value as i32)
                    .unwrap_or(-1)
            }
            AttributeDomain::Detail => -1,
        }
    }

    fn current_vtxnum(&mut self, idx: usize) -> i32 {
        match self.domain {
            AttributeDomain::Vertex => idx as i32,
            AttributeDomain::Point => {
                self.ensure_point_maps();
                self.point_first_vertex
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .filter(|value| *value != usize::MAX)
                    .map(|value| value as i32)
                    .unwrap_or(-1)
            }
            AttributeDomain::Primitive => {
                let base = idx * 3;
                if base < self.mesh.indices.len() {
                    base as i32
                } else {
                    -1
                }
            }
            AttributeDomain::Detail => -1,
        }
    }

    fn current_primnum(&mut self, idx: usize) -> i32 {
        match self.domain {
            AttributeDomain::Primitive => idx as i32,
            AttributeDomain::Vertex => (idx / 3) as i32,
            AttributeDomain::Point => {
                self.ensure_point_maps();
                self.point_first_prim
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .filter(|value| *value != usize::MAX)
                    .map(|value| value as i32)
                    .unwrap_or(-1)
            }
            AttributeDomain::Detail => -1,
        }
    }

    fn read_p(&mut self, idx: usize) -> [f32; 3] {
        self.read_p_for_domain(self.domain, idx)
    }

    fn read_n(&mut self, idx: usize) -> [f32; 3] {
        self.read_n_for_domain(self.domain, idx)
    }

    fn read_p_for_domain(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point => self.mesh.positions.get(idx).copied().unwrap_or([0.0; 3]),
            AttributeDomain::Vertex => {
                let point = self.mesh.indices.get(idx).copied().unwrap_or(0) as usize;
                self.mesh.positions.get(point).copied().unwrap_or([0.0; 3])
            }
            AttributeDomain::Primitive => {
                self.ensure_prim_centers();
                self.prim_centers
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0; 3])
            }
            AttributeDomain::Detail => {
                self.ensure_detail_center();
                self.detail_center.unwrap_or([0.0; 3])
            }
        }
    }

    fn read_n_for_domain(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point => {
                self.ensure_point_normals();
                self.point_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Vertex => {
                self.ensure_vertex_normals();
                self.vertex_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Primitive => {
                self.ensure_prim_normals();
                self.prim_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Detail => {
                self.ensure_detail_normal();
                self.detail_normal.unwrap_or([0.0, 1.0, 0.0])
            }
        }
    }

    fn ensure_point_normals(&mut self) {
        if self.point_normals.is_some() {
            return;
        }
        if let Some(normals) = &self.mesh.normals {
            self.point_normals = Some(normals.clone());
            return;
        }
        self.point_normals = Some(compute_point_normals(self.mesh));
    }

    fn ensure_vertex_normals(&mut self) {
        if self.vertex_normals.is_some() {
            return;
        }
        if let Some(normals) = &self.mesh.corner_normals {
            self.vertex_normals = Some(normals.clone());
            return;
        }
        self.ensure_point_normals();
        let point_normals = self.point_normals.as_ref().cloned().unwrap_or_default();
        let mut result = Vec::with_capacity(self.mesh.indices.len());
        for idx in &self.mesh.indices {
            let point = *idx as usize;
            result.push(point_normals.get(point).copied().unwrap_or([0.0, 1.0, 0.0]));
        }
        self.vertex_normals = Some(result);
    }

    fn ensure_prim_normals(&mut self) {
        if self.prim_normals.is_some() {
            return;
        }
        let mut normals = Vec::new();
        for tri in self.mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            let p0 = Vec3::from(self.mesh.positions.get(i0).copied().unwrap_or([0.0; 3]));
            let p1 = Vec3::from(self.mesh.positions.get(i1).copied().unwrap_or([0.0; 3]));
            let p2 = Vec3::from(self.mesh.positions.get(i2).copied().unwrap_or([0.0; 3]));
            let n = (p1 - p0).cross(p2 - p0);
            let n = if n.length_squared() > 0.0 {
                n.normalize().to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
            normals.push(n);
        }
        self.prim_normals = Some(normals);
    }

    fn ensure_prim_centers(&mut self) {
        if self.prim_centers.is_some() {
            return;
        }
        let mut centers = Vec::new();
        for tri in self.mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            let p0 = Vec3::from(self.mesh.positions.get(i0).copied().unwrap_or([0.0; 3]));
            let p1 = Vec3::from(self.mesh.positions.get(i1).copied().unwrap_or([0.0; 3]));
            let p2 = Vec3::from(self.mesh.positions.get(i2).copied().unwrap_or([0.0; 3]));
            let center = (p0 + p1 + p2) / 3.0;
            centers.push(center.to_array());
        }
        self.prim_centers = Some(centers);
    }

    fn ensure_detail_center(&mut self) {
        if self.detail_center.is_some() {
            return;
        }
        let center = self.mesh.bounds().map(|bounds| {
            [
                (bounds.min[0] + bounds.max[0]) * 0.5,
                (bounds.min[1] + bounds.max[1]) * 0.5,
                (bounds.min[2] + bounds.max[2]) * 0.5,
            ]
        });
        self.detail_center = Some(center.unwrap_or([0.0; 3]));
    }

    fn ensure_detail_normal(&mut self) {
        if self.detail_normal.is_some() {
            return;
        }
        self.ensure_point_normals();
        let normals = self.point_normals.as_ref().cloned().unwrap_or_default();
        let mut sum = Vec3::ZERO;
        for n in normals {
            sum += Vec3::from(n);
        }
        let normal = if sum.length_squared() > 0.0 {
            sum.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
        self.detail_normal = Some(normal);
    }

    fn ensure_point_maps(&mut self) {
        if self.point_first_vertex.is_some() && self.point_first_prim.is_some() {
            return;
        }
        let mut first_vertex = vec![usize::MAX; self.mesh.positions.len()];
        let mut first_prim = vec![usize::MAX; self.mesh.positions.len()];
        for (vertex_index, point_index) in self.mesh.indices.iter().enumerate() {
            let point_index = *point_index as usize;
            if let Some(slot) = first_vertex.get_mut(point_index) {
                if *slot == usize::MAX {
                    *slot = vertex_index;
                }
            }
            if let Some(slot) = first_prim.get_mut(point_index) {
                if *slot == usize::MAX {
                    *slot = vertex_index / 3;
                }
            }
        }
        self.point_first_vertex = Some(first_vertex);
        self.point_first_prim = Some(first_prim);
    }
}

struct SplatQueryCache<'a> {
    splats: &'a SplatGeo,
    point_normals: Option<Vec<[f32; 3]>>,
    detail_center: Option<[f32; 3]>,
    detail_normal: Option<[f32; 3]>,
}

impl<'a> SplatQueryCache<'a> {
    fn new(splats: &'a SplatGeo) -> Self {
        Self {
            splats,
            point_normals: None,
            detail_center: None,
            detail_normal: None,
        }
    }

    fn read_p(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => self
                .splats
                .positions
                .get(idx)
                .copied()
                .unwrap_or([0.0; 3]),
            AttributeDomain::Detail => {
                self.ensure_detail_center();
                self.detail_center.unwrap_or([0.0; 3])
            }
            AttributeDomain::Vertex => [0.0; 3],
        }
    }

    fn read_n(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                self.ensure_point_normals();
                self.point_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Detail => {
                self.ensure_detail_normal();
                self.detail_normal.unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Vertex => [0.0, 1.0, 0.0],
        }
    }

    fn ensure_point_normals(&mut self) {
        if self.point_normals.is_some() {
            return;
        }
        if let Some(AttributeRef::Vec3(values)) =
            self.splats.attribute(AttributeDomain::Point, "N")
        {
            if values.len() == self.splats.positions.len() {
                self.point_normals = Some(values.to_vec());
                return;
            }
        }
        self.point_normals =
            Some(vec![[0.0, 1.0, 0.0]; self.splats.positions.len()]);
    }

    fn ensure_detail_center(&mut self) {
        if self.detail_center.is_some() {
            return;
        }
        let mut iter = self.splats.positions.iter();
        let Some(first) = iter.next().copied() else {
            self.detail_center = Some([0.0; 3]);
            return;
        };
        let mut min = first;
        let mut max = first;
        for p in iter {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        }
        self.detail_center = Some([
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]);
    }

    fn ensure_detail_normal(&mut self) {
        if self.detail_normal.is_some() {
            return;
        }
        self.ensure_point_normals();
        let normals = self.point_normals.as_ref().cloned().unwrap_or_default();
        let mut sum = Vec3::ZERO;
        for n in normals {
            sum += Vec3::from(n);
        }
        let normal = if sum.length_squared() > 0.0 {
            sum.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
        self.detail_normal = Some(normal);
    }
}

struct SplatWrangleContext<'a> {
    splats: &'a SplatGeo,
    secondary_query: Option<SplatQueryCache<'a>>,
    primary_volume: Option<VolumeSampler<'a>>,
    secondary_volume: Option<VolumeSampler<'a>>,
    domain: AttributeDomain,
    len: usize,
    mask: Option<&'a [bool]>,
    written: HashMap<String, AttributeStorage>,
    point_normals: Option<Vec<[f32; 3]>>,
    detail_center: Option<[f32; 3]>,
    detail_normal: Option<[f32; 3]>,
}

impl<'a> SplatWrangleContext<'a> {
    fn new(
        splats: &'a SplatGeo,
        domain: AttributeDomain,
        mask: Option<&'a [bool]>,
        secondary_splats: Option<&'a SplatGeo>,
        primary_volume: Option<&'a Volume>,
        secondary_volume: Option<&'a Volume>,
    ) -> Self {
        let len = splats.attribute_domain_len(domain);
        Self {
            splats,
            secondary_query: secondary_splats.map(SplatQueryCache::new),
            primary_volume: primary_volume.map(VolumeSampler::new),
            secondary_volume: secondary_volume.map(VolumeSampler::new),
            domain,
            len,
            mask,
            written: HashMap::new(),
            point_normals: None,
            detail_center: None,
            detail_normal: None,
        }
    }

    fn apply_statement(&mut self, stmt: Statement) -> Result<(), String> {
        match stmt {
            Statement::Assign { target, expr } => self.assign(target, expr),
        }
    }

    fn assign(&mut self, target: String, expr: Expr) -> Result<(), String> {
        if target == "P" && self.domain != AttributeDomain::Point {
            return Err("Wrangle can only write @P in Point mode".to_string());
        }
        if self.len == 0 {
            return Ok(());
        }

        if self.mask.is_some() && !self.any_selected() {
            return Ok(());
        }

        let target_type = self.target_type(&target).or_else(|| {
            let idx = self.first_selected_index().unwrap_or(0);
            self.eval_expr(&expr, idx).ok().map(|value| value.data_type())
        });

        let mut values = Vec::with_capacity(self.len.max(1));
        for idx in 0..self.len.max(1) {
            let selected = self
                .mask
                .and_then(|mask| mask.get(idx).copied())
                .unwrap_or(true);
            let value = if selected {
                self.eval_expr(&expr, idx)?
            } else {
                self.read_attr_for_mask(&target, idx, target_type)?
            };
            values.push(value);
        }

        let storage = build_storage(&values, target_type)?;
        self.written.insert(target, storage);
        Ok(())
    }

    fn into_written(self) -> HashMap<String, AttributeStorage> {
        self.written
    }

    fn target_type(&self, name: &str) -> Option<AttributeType> {
        if let Some(storage) = self.written.get(name) {
            return Some(storage.data_type());
        }
        match (name, self.domain) {
            ("P", AttributeDomain::Point) => return Some(AttributeType::Vec3),
            ("N", AttributeDomain::Point) => return Some(AttributeType::Vec3),
            _ => {}
        }
        self.splats
            .attribute(self.domain, name)
            .map(|attr| attr.data_type())
    }

    fn eval_expr(&mut self, expr: &Expr, idx: usize) -> Result<Value, String> {
        match expr {
            Expr::Literal(value) => Ok(*value),
            Expr::Attr(name) => self.read_attr(name, idx),
            Expr::Ident(name) => Err(format!("Unknown identifier '{}'", name)),
            Expr::Swizzle { expr, mask } => {
                let value = self.eval_expr(expr, idx)?;
                swizzle_value(value, mask)
            }
            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr, idx)?;
                Ok(match op {
                    UnaryOp::Pos => value,
                    UnaryOp::Neg => value.negate(),
                })
            }
            Expr::Binary { op, left, right } => {
                let a = self.eval_expr(left, idx)?;
                let b = self.eval_expr(right, idx)?;
                match op {
                    BinaryOp::Add => add_values(a, b),
                    BinaryOp::Sub => sub_values(a, b),
                    BinaryOp::Mul => mul_values(a, b),
                    BinaryOp::Div => div_values(a, b),
                }
            }
            Expr::Call { name, args } => self.eval_call(name, args, idx),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr], idx: usize) -> Result<Value, String> {
        let name = name.to_lowercase();
        match name.as_str() {
            "sin" | "cos" | "tan" | "abs" | "floor" | "ceil" => {
                let value = self.eval_args(args, idx, 1)?[0];
                Ok(match name.as_str() {
                    "sin" => map_value(value, f32::sin),
                    "cos" => map_value(value, f32::cos),
                    "tan" => map_value(value, f32::tan),
                    "abs" => map_value(value, f32::abs),
                    "floor" => map_value(value, f32::floor),
                    _ => map_value(value, f32::ceil),
                })
            }
            "pow" => {
                let values = self.eval_args(args, idx, 2)?;
                pow_values(values[0], values[1])
            }
            "min" => {
                let values = self.eval_args(args, idx, 2)?;
                min_values(values[0], values[1])
            }
            "max" => {
                let values = self.eval_args(args, idx, 2)?;
                max_values(values[0], values[1])
            }
            "clamp" => {
                let values = self.eval_args(args, idx, 3)?;
                clamp_values(values[0], values[1], values[2])
            }
            "lerp" => {
                let values = self.eval_args(args, idx, 3)?;
                lerp_values(values[0], values[1], values[2])
            }
            "len" => {
                let value = self.eval_args(args, idx, 1)?[0];
                Ok(Value::Float(length_value(value)))
            }
            "dot" => {
                let values = self.eval_args(args, idx, 2)?;
                let dot = dot_values(values[0], values[1])?;
                Ok(Value::Float(dot))
            }
            "normalize" => {
                let value = self.eval_args(args, idx, 1)?[0];
                normalize_value(value)
            }
            "point" => self.eval_geo_query(AttributeDomain::Point, args, idx),
            "vertex" => self.eval_geo_query(AttributeDomain::Vertex, args, idx),
            "prim" => self.eval_geo_query(AttributeDomain::Primitive, args, idx),
            "splat" => self.eval_splat_query(args, idx),
            "sample" => self.eval_volume_sample(args, idx),
            "vec2" => build_vec_splats(args, idx, 2, self),
            "vec3" => build_vec_splats(args, idx, 3, self),
            "vec4" => build_vec_splats(args, idx, 4, self),
            _ => Err(format!("Unknown function '{}'", name)),
        }
    }

    fn eval_args(
        &mut self,
        args: &[Expr],
        idx: usize,
        expected: usize,
    ) -> Result<Vec<Value>, String> {
        if args.len() != expected {
            return Err(format!(
                "Expected {} argument(s), got {}",
                expected,
                args.len()
            ));
        }
        let mut out = Vec::with_capacity(args.len());
        for arg in args {
            out.push(self.eval_expr(arg, idx)?);
        }
        Ok(out)
    }

    fn eval_splat_query(&mut self, args: &[Expr], idx: usize) -> Result<Value, String> {
        if args.len() != 3 {
            return Err(format!("Expected 3 arguments, got {}", args.len()));
        }
        let input_index = value_to_index(self.eval_expr(&args[0], idx)?)?;
        let attr_name = attr_name_arg(&args[1])?;
        let elem_index = value_to_index(self.eval_expr(&args[2], idx)?)?;

        match input_index {
            0 => Ok(self.query_primary_splat_attr(&attr_name, elem_index)),
            1 => Ok(self.query_secondary_splat_attr(&attr_name, elem_index)),
            _ => Err("Input index must be 0 or 1".to_string()),
        }
    }

    fn eval_geo_query(
        &mut self,
        domain: AttributeDomain,
        args: &[Expr],
        idx: usize,
    ) -> Result<Value, String> {
        if args.len() != 3 {
            return Err(format!("Expected 3 arguments, got {}", args.len()));
        }
        let input_index = value_to_index(self.eval_expr(&args[0], idx)?)?;
        let attr_name = attr_name_arg(&args[1])?;
        let elem_index = value_to_index(self.eval_expr(&args[2], idx)?)?;

        match input_index {
            0 => Ok(self.query_primary_attr(domain, &attr_name, elem_index)),
            1 => Ok(self.query_secondary_attr(domain, &attr_name, elem_index)),
            _ => Err("Input index must be 0 or 1".to_string()),
        }
    }

    fn eval_volume_sample(&mut self, args: &[Expr], idx: usize) -> Result<Value, String> {
        let (input_index, pos_expr) = match args.len() {
            2 => (self.eval_expr(&args[0], idx)?, &args[1]),
            3 => {
                let _ = attr_name_arg(&args[1])?;
                (self.eval_expr(&args[0], idx)?, &args[2])
            }
            _ => {
                return Err(format!(
                    "sample expects 2 or 3 arguments, got {}",
                    args.len()
                ));
            }
        };
        let input_index = value_to_index(input_index)?;
        let pos_value = self.eval_expr(pos_expr, idx)?;
        let pos = value_to_vec3(pos_value)?;

        let sampler = match input_index {
            0 => self.primary_volume.as_ref(),
            1 => self.secondary_volume.as_ref(),
            _ => return Err("Input index must be 0 or 1".to_string()),
        };
        Ok(Value::Float(
            sampler.map(|sampler| sampler.sample_world(pos)).unwrap_or(0.0),
        ))
    }

    fn query_primary_splat_attr(&mut self, name: &str, idx: usize) -> Value {
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(self.read_p_for_domain(AttributeDomain::Point, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(self.read_n_for_domain(AttributeDomain::Point, idx));
        }
        if let Some(attr) = self.splats.attribute(AttributeDomain::Point, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = self.splats.attribute(AttributeDomain::Primitive, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = self.splats.attribute(AttributeDomain::Detail, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn query_secondary_splat_attr(&mut self, name: &str, idx: usize) -> Value {
        let Some(query) = self.secondary_query.as_mut() else {
            return default_query_value(name);
        };
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(query.read_p(AttributeDomain::Point, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(query.read_n(AttributeDomain::Point, idx));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Point, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Primitive, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        if let Some(attr) = query.splats.attribute(AttributeDomain::Detail, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn query_primary_attr(
        &mut self,
        domain: AttributeDomain,
        name: &str,
        idx: usize,
    ) -> Value {
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(self.read_p_for_domain(domain, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(self.read_n_for_domain(domain, idx));
        }
        if let Some(attr) = self.splats.attribute(domain, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn query_secondary_attr(
        &mut self,
        domain: AttributeDomain,
        name: &str,
        idx: usize,
    ) -> Value {
        let Some(query) = self.secondary_query.as_mut() else {
            return default_query_value(name);
        };
        if name.eq_ignore_ascii_case("P") {
            return Value::Vec3(query.read_p(domain, idx));
        }
        if name.eq_ignore_ascii_case("N") {
            return Value::Vec3(query.read_n(domain, idx));
        }
        if let Some(attr) = query.splats.attribute(domain, name) {
            return value_from_attr_ref(attr, idx).unwrap_or(Value::Float(0.0));
        }
        default_query_value(name)
    }

    fn read_attr(&mut self, name: &str, idx: usize) -> Result<Value, String> {
        if let Some(storage) = self.written.get(name) {
            return value_from_storage(storage, idx);
        }
        if let Some(value) = self.read_implicit_attr(name, idx) {
            return Ok(value);
        }
        if name == "P" {
            return Ok(Value::Vec3(self.read_p(idx)));
        }
        if name == "N" {
            return Ok(Value::Vec3(self.read_n(idx)));
        }
        if let Some(attr) = self.splats.attribute(self.domain, name) {
            return value_from_attr_ref(attr, idx);
        }
        Ok(Value::Float(0.0))
    }

    fn read_attr_for_mask(
        &mut self,
        name: &str,
        idx: usize,
        target_type: Option<AttributeType>,
    ) -> Result<Value, String> {
        if let Some(target_type) = target_type {
            if let Some(attr) = self.splats.attribute(self.domain, name) {
                if attr.data_type() == target_type {
                    return value_from_attr_ref(attr, idx);
                }
            }
            if name == "P" && target_type == AttributeType::Vec3 {
                return Ok(Value::Vec3(self.read_p(idx)));
            }
            if name == "N" && target_type == AttributeType::Vec3 {
                return Ok(Value::Vec3(self.read_n(idx)));
            }
            return Ok(default_value_for_type(target_type));
        }
        self.read_attr(name, idx)
    }

    fn first_selected_index(&self) -> Option<usize> {
        let mask = self.mask?;
        mask.iter().position(|value| *value)
    }

    fn any_selected(&self) -> bool {
        let Some(mask) = self.mask else {
            return true;
        };
        mask.iter().any(|value| *value)
    }

    fn read_implicit_attr(&self, name: &str, idx: usize) -> Option<Value> {
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "ptnum" => Some(Value::Float(self.current_ptnum(idx) as f32)),
            "vtxnum" => Some(Value::Float(self.current_vtxnum(idx) as f32)),
            "primnum" => Some(Value::Float(self.current_primnum(idx) as f32)),
            "numpt" => Some(Value::Float(self.splats.positions.len() as f32)),
            "numvtx" => Some(Value::Float(0.0)),
            "numprim" => Some(Value::Float(self.splats.positions.len() as f32)),
            _ => None,
        }
    }

    fn current_ptnum(&self, idx: usize) -> i32 {
        match self.domain {
            AttributeDomain::Point | AttributeDomain::Primitive => idx as i32,
            AttributeDomain::Detail | AttributeDomain::Vertex => -1,
        }
    }

    fn current_vtxnum(&self, _idx: usize) -> i32 {
        -1
    }

    fn current_primnum(&self, idx: usize) -> i32 {
        match self.domain {
            AttributeDomain::Point | AttributeDomain::Primitive => idx as i32,
            AttributeDomain::Detail | AttributeDomain::Vertex => -1,
        }
    }

    fn read_p(&mut self, idx: usize) -> [f32; 3] {
        self.read_p_for_domain(self.domain, idx)
    }

    fn read_n(&mut self, idx: usize) -> [f32; 3] {
        self.read_n_for_domain(self.domain, idx)
    }

    fn read_p_for_domain(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => self
                .splats
                .positions
                .get(idx)
                .copied()
                .unwrap_or([0.0; 3]),
            AttributeDomain::Detail => {
                self.ensure_detail_center();
                self.detail_center.unwrap_or([0.0; 3])
            }
            AttributeDomain::Vertex => [0.0; 3],
        }
    }

    fn read_n_for_domain(&mut self, domain: AttributeDomain, idx: usize) -> [f32; 3] {
        match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                self.ensure_point_normals();
                self.point_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Detail => {
                self.ensure_detail_normal();
                self.detail_normal.unwrap_or([0.0, 1.0, 0.0])
            }
            AttributeDomain::Vertex => [0.0, 1.0, 0.0],
        }
    }

    fn ensure_point_normals(&mut self) {
        if self.point_normals.is_some() {
            return;
        }
        if let Some(AttributeRef::Vec3(values)) =
            self.splats.attribute(AttributeDomain::Point, "N")
        {
            if values.len() == self.splats.positions.len() {
                self.point_normals = Some(values.to_vec());
                return;
            }
        }
        self.point_normals =
            Some(vec![[0.0, 1.0, 0.0]; self.splats.positions.len()]);
    }

    fn ensure_detail_center(&mut self) {
        if self.detail_center.is_some() {
            return;
        }
        let mut iter = self.splats.positions.iter();
        let Some(first) = iter.next().copied() else {
            self.detail_center = Some([0.0; 3]);
            return;
        };
        let mut min = first;
        let mut max = first;
        for p in iter {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        }
        self.detail_center = Some([
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]);
    }

    fn ensure_detail_normal(&mut self) {
        if self.detail_normal.is_some() {
            return;
        }
        self.ensure_point_normals();
        let normals = self.point_normals.as_ref().cloned().unwrap_or_default();
        let mut sum = Vec3::ZERO;
        for n in normals {
            sum += Vec3::from(n);
        }
        let normal = if sum.length_squared() > 0.0 {
            sum.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
        self.detail_normal = Some(normal);
    }
}

fn value_from_attr_ref(attr: AttributeRef<'_>, idx: usize) -> Result<Value, String> {
    match attr {
        AttributeRef::Float(values) => Ok(Value::Float(values.get(idx).copied().unwrap_or(0.0))),
        AttributeRef::Int(values) => Ok(Value::Float(values.get(idx).copied().unwrap_or(0) as f32)),
        AttributeRef::Vec2(values) => Ok(Value::Vec2(values.get(idx).copied().unwrap_or([0.0; 2]))),
        AttributeRef::Vec3(values) => Ok(Value::Vec3(values.get(idx).copied().unwrap_or([0.0; 3]))),
        AttributeRef::Vec4(values) => Ok(Value::Vec4(values.get(idx).copied().unwrap_or([0.0; 4]))),
        AttributeRef::StringTable(_) => {
            Err("Wrangle does not support string attributes".to_string())
        }
    }
}

fn attr_name_arg(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Ident(name) => Ok(name.clone()),
        _ => Err("Attribute name must be an identifier".to_string()),
    }
}

fn value_to_index(value: Value) -> Result<usize, String> {
    match value {
        Value::Float(v) => {
            if !v.is_finite() {
                return Err("Index argument must be finite".to_string());
            }
            let idx = v.round() as i64;
            if idx < 0 {
                return Err("Index argument must be >= 0".to_string());
            }
            Ok(idx as usize)
        }
        _ => Err("Index argument must be a number".to_string()),
    }
}

fn value_to_vec3(value: Value) -> Result<Vec3, String> {
    match value {
        Value::Vec3(v) => Ok(Vec3::from(v)),
        _ => Err("Position argument must be a vec3".to_string()),
    }
}

fn default_query_value(name: &str) -> Value {
    if name.eq_ignore_ascii_case("P") || name.eq_ignore_ascii_case("N") {
        Value::Vec3([0.0, 0.0, 0.0])
    } else {
        Value::Float(0.0)
    }
}

fn value_from_storage(storage: &AttributeStorage, idx: usize) -> Result<Value, String> {
    match storage {
        AttributeStorage::Float(values) => {
            Ok(Value::Float(values.get(idx).copied().unwrap_or(0.0)))
        }
        AttributeStorage::Int(values) => {
            Ok(Value::Float(values.get(idx).copied().unwrap_or(0) as f32))
        }
        AttributeStorage::Vec2(values) => {
            Ok(Value::Vec2(values.get(idx).copied().unwrap_or([0.0; 2])))
        }
        AttributeStorage::Vec3(values) => {
            Ok(Value::Vec3(values.get(idx).copied().unwrap_or([0.0; 3])))
        }
        AttributeStorage::Vec4(values) => {
            Ok(Value::Vec4(values.get(idx).copied().unwrap_or([0.0; 4])))
        }
        AttributeStorage::StringTable(_) => {
            Err("Wrangle does not support string attributes".to_string())
        }
    }
}

fn build_storage(
    values: &[Value],
    target: Option<AttributeType>,
) -> Result<AttributeStorage, String> {
    let first = values.first().copied().unwrap_or(Value::Float(0.0));
    let target_type = target.unwrap_or(first.data_type());
    match target_type {
        AttributeType::Float => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                match value {
                    Value::Float(v) => out.push(*v),
                    _ => return Err("Cannot assign vector to float attribute".to_string()),
                }
            }
            Ok(AttributeStorage::Float(out))
        }
        AttributeType::Int => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                match value {
                    Value::Float(v) => out.push(v.round() as i32),
                    _ => return Err("Cannot assign vector to int attribute".to_string()),
                }
            }
            Ok(AttributeStorage::Int(out))
        }
        AttributeType::Vec2 => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                out.push(match value {
                    Value::Vec2(v) => *v,
                    Value::Float(v) => [*v; 2],
                    _ => return Err("Cannot assign Vec3/Vec4 to vec2 attribute".to_string()),
                });
            }
            Ok(AttributeStorage::Vec2(out))
        }
        AttributeType::Vec3 => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                out.push(match value {
                    Value::Vec3(v) => *v,
                    Value::Float(v) => [*v; 3],
                    _ => return Err("Cannot assign Vec2/Vec4 to vec3 attribute".to_string()),
                });
            }
            Ok(AttributeStorage::Vec3(out))
        }
        AttributeType::Vec4 => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                out.push(match value {
                    Value::Vec4(v) => *v,
                    Value::Float(v) => [*v; 4],
                    _ => return Err("Cannot assign Vec2/Vec3 to vec4 attribute".to_string()),
                });
            }
            Ok(AttributeStorage::Vec4(out))
        }
        AttributeType::String => Err("Wrangle does not support string attributes".to_string()),
    }
}

fn default_value_for_type(target_type: AttributeType) -> Value {
    match target_type {
        AttributeType::Float => Value::Float(0.0),
        AttributeType::Int => Value::Float(0.0),
        AttributeType::Vec2 => Value::Vec2([0.0, 0.0]),
        AttributeType::Vec3 => Value::Vec3([0.0, 0.0, 0.0]),
        AttributeType::Vec4 => Value::Vec4([0.0, 0.0, 0.0, 0.0]),
        AttributeType::String => Value::Float(0.0),
    }
}

fn apply_written(
    mesh: &mut Mesh,
    domain: AttributeDomain,
    written: HashMap<String, AttributeStorage>,
) -> Result<(), String> {
    for (name, storage) in written {
        mesh.set_attribute(domain, name, storage)
            .map_err(|err| format!("Wrangle attribute error: {:?}", err))?;
    }
    Ok(())
}

fn apply_written_splats(
    splats: &mut SplatGeo,
    domain: AttributeDomain,
    written: HashMap<String, AttributeStorage>,
) -> Result<(), String> {
    for (name, storage) in written {
        splats
            .set_attribute(domain, name, storage)
            .map_err(|err| format!("Wrangle attribute error: {:?}", err))?;
    }
    Ok(())
}

fn compute_point_normals(mesh: &Mesh) -> Vec<[f32; 3]> {
    if !mesh.indices.len().is_multiple_of(3) || mesh.positions.is_empty() {
        return vec![];
    }
    let mut accum = vec![Vec3::ZERO; mesh.positions.len()];
    for tri in mesh.indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = Vec3::from(mesh.positions.get(i0).copied().unwrap_or([0.0; 3]));
        let p1 = Vec3::from(mesh.positions.get(i1).copied().unwrap_or([0.0; 3]));
        let p2 = Vec3::from(mesh.positions.get(i2).copied().unwrap_or([0.0; 3]));
        let normal = (p1 - p0).cross(p2 - p0);
        accum[i0] += normal;
        accum[i1] += normal;
        accum[i2] += normal;
    }

    accum
        .into_iter()
        .map(|n| {
            let len = n.length();
            if len > 0.0 {
                (n / len).to_array()
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect()
}

fn map_value(value: Value, f: impl Fn(f32) -> f32) -> Value {
    match value {
        Value::Float(v) => Value::Float(f(v)),
        Value::Vec2(v) => Value::Vec2([f(v[0]), f(v[1])]),
        Value::Vec3(v) => Value::Vec3([f(v[0]), f(v[1]), f(v[2])]),
        Value::Vec4(v) => Value::Vec4([f(v[0]), f(v[1]), f(v[2]), f(v[3])]),
    }
}

fn length_value(value: Value) -> f32 {
    match value {
        Value::Float(v) => v.abs(),
        Value::Vec2(v) => (v[0] * v[0] + v[1] * v[1]).sqrt(),
        Value::Vec3(v) => (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(),
        Value::Vec4(v) => (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3]).sqrt(),
    }
}

fn dot_values(a: Value, b: Value) -> Result<f32, String> {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => Ok(a * b),
        (Value::Vec2(a), Value::Vec2(b)) => Ok(a[0] * b[0] + a[1] * b[1]),
        (Value::Vec3(a), Value::Vec3(b)) => Ok(a[0] * b[0] + a[1] * b[1] + a[2] * b[2]),
        (Value::Vec4(a), Value::Vec4(b)) => {
            Ok(a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3])
        }
        _ => Err("dot expects two values of the same size".to_string()),
    }
}

fn normalize_value(value: Value) -> Result<Value, String> {
    match value {
        Value::Float(_) => Err("normalize expects a vector".to_string()),
        Value::Vec2(v) => {
            let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
            if len > 0.0 {
                Ok(Value::Vec2([v[0] / len, v[1] / len]))
            } else {
                Ok(Value::Vec2(v))
            }
        }
        Value::Vec3(v) => {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if len > 0.0 {
                Ok(Value::Vec3([v[0] / len, v[1] / len, v[2] / len]))
            } else {
                Ok(Value::Vec3(v))
            }
        }
        Value::Vec4(v) => {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3]).sqrt();
            if len > 0.0 {
                Ok(Value::Vec4([
                    v[0] / len,
                    v[1] / len,
                    v[2] / len,
                    v[3] / len,
                ]))
            } else {
                Ok(Value::Vec4(v))
            }
        }
    }
}

fn swizzle_value(value: Value, mask: &str) -> Result<Value, String> {
    let mask = mask.to_ascii_lowercase();
    let count = mask.chars().count();
    if count == 0 || count > 4 {
        return Err("Swizzle mask must be 1-4 characters".to_string());
    }
    let mut indices = Vec::with_capacity(count);
    for ch in mask.chars() {
        let idx = match ch {
            'x' | 'r' => 0,
            'y' | 'g' => 1,
            'z' | 'b' => 2,
            'w' | 'a' => 3,
            _ => return Err(format!("Invalid swizzle component '{}'", ch)),
        };
        indices.push(idx);
    }
    match value {
        Value::Float(_) => Err("Swizzle expects a vector".to_string()),
        Value::Vec2(v) => swizzle_from_slice(&v, indices),
        Value::Vec3(v) => swizzle_from_slice(&v, indices),
        Value::Vec4(v) => swizzle_from_slice(&v, indices),
    }
}

fn swizzle_from_slice(values: &[f32], indices: Vec<usize>) -> Result<Value, String> {
    for &idx in &indices {
        if idx >= values.len() {
            return Err("Swizzle component out of range".to_string());
        }
    }
    Ok(match indices.len() {
        1 => Value::Float(values[indices[0]]),
        2 => Value::Vec2([values[indices[0]], values[indices[1]]]),
        3 => Value::Vec3([values[indices[0]], values[indices[1]], values[indices[2]]]),
        4 => Value::Vec4([
            values[indices[0]],
            values[indices[1]],
            values[indices[2]],
            values[indices[3]],
        ]),
        _ => return Err("Swizzle mask must be 1-4 characters".to_string()),
    })
}

fn safe_div(a: f32, b: f32) -> f32 {
    if b.abs() < 1.0e-6 {
        a
    } else {
        a / b
    }
}

fn add_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, |x, y| x + y)
}

fn sub_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, |x, y| x - y)
}

fn mul_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, |x, y| x * y)
}

fn div_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, safe_div)
}

fn min_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, f32::min)
}

fn max_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, f32::max)
}

fn clamp_values(value: Value, lo: Value, hi: Value) -> Result<Value, String> {
    let clamped = max_values(value, lo)?;
    min_values(clamped, hi)
}

fn lerp_values(a: Value, b: Value, t: Value) -> Result<Value, String> {
    let t = match t {
        Value::Float(v) => v,
        _ => return Err("lerp expects a float t".to_string()),
    };
    let diff = sub_values(b, a)?;
    let scaled = mul_values(diff, Value::Float(t))?;
    add_values(a, scaled)
}

fn pow_values(a: Value, b: Value) -> Result<Value, String> {
    binary_op(a, b, f32::powf)
}

fn binary_op(a: Value, b: Value, f: impl Fn(f32, f32) -> f32) -> Result<Value, String> {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(f(a, b))),
        (Value::Vec2(a), Value::Vec2(b)) => Ok(Value::Vec2([f(a[0], b[0]), f(a[1], b[1])])),
        (Value::Vec3(a), Value::Vec3(b)) => {
            Ok(Value::Vec3([f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]))
        }
        (Value::Vec4(a), Value::Vec4(b)) => Ok(Value::Vec4([
            f(a[0], b[0]),
            f(a[1], b[1]),
            f(a[2], b[2]),
            f(a[3], b[3]),
        ])),
        (Value::Float(a), Value::Vec2(b)) => Ok(Value::Vec2([f(a, b[0]), f(a, b[1])])),
        (Value::Vec2(a), Value::Float(b)) => Ok(Value::Vec2([f(a[0], b), f(a[1], b)])),
        (Value::Float(a), Value::Vec3(b)) => Ok(Value::Vec3([f(a, b[0]), f(a, b[1]), f(a, b[2])])),
        (Value::Vec3(a), Value::Float(b)) => Ok(Value::Vec3([f(a[0], b), f(a[1], b), f(a[2], b)])),
        (Value::Float(a), Value::Vec4(b)) => Ok(Value::Vec4([
            f(a, b[0]),
            f(a, b[1]),
            f(a, b[2]),
            f(a, b[3]),
        ])),
        (Value::Vec4(a), Value::Float(b)) => Ok(Value::Vec4([
            f(a[0], b),
            f(a[1], b),
            f(a[2], b),
            f(a[3], b),
        ])),
        _ => Err("Vector size mismatch".to_string()),
    }
}

fn build_vec(
    args: &[Expr],
    idx: usize,
    size: usize,
    ctx: &mut WrangleContext<'_>,
) -> Result<Value, String> {
    let values = if args.len() == 1 {
        vec![ctx.eval_expr(&args[0], idx)?; size]
    } else if args.len() == size {
        let mut out = Vec::with_capacity(size);
        for arg in args {
            out.push(ctx.eval_expr(arg, idx)?);
        }
        out
    } else {
        return Err(format!("vec{} expects 1 or {} arguments", size, size));
    };

    let mut floats = Vec::with_capacity(size);
    for value in values {
        match value {
            Value::Float(v) => floats.push(v),
            _ => return Err("vec* arguments must be floats".to_string()),
        }
    }

    Ok(match size {
        2 => Value::Vec2([floats[0], floats[1]]),
        3 => Value::Vec3([floats[0], floats[1], floats[2]]),
        4 => Value::Vec4([floats[0], floats[1], floats[2], floats[3]]),
        _ => Value::Float(0.0),
    })
}

fn build_vec_splats(
    args: &[Expr],
    idx: usize,
    size: usize,
    ctx: &mut SplatWrangleContext<'_>,
) -> Result<Value, String> {
    let values = if args.len() == 1 {
        vec![ctx.eval_expr(&args[0], idx)?; size]
    } else if args.len() == size {
        let mut out = Vec::with_capacity(size);
        for arg in args {
            out.push(ctx.eval_expr(arg, idx)?);
        }
        out
    } else {
        return Err(format!("vec{} expects 1 or {} arguments", size, size));
    };

    let mut floats = Vec::with_capacity(size);
    for value in values {
        match value {
            Value::Float(v) => floats.push(v),
            _ => return Err("vec* arguments must be floats".to_string()),
        }
    }

    Ok(match size {
        2 => Value::Vec2([floats[0], floats[1]]),
        3 => Value::Vec3([floats[0], floats[1], floats[2]]),
        4 => Value::Vec4([floats[0], floats[1], floats[2], floats[3]]),
        _ => Value::Float(0.0),
    })
}

fn parse_program(code: &str) -> Result<Program, String> {
    let tokens = tokenize(code)?;
    let mut parser = Parser::new(tokens);
    let mut statements = Vec::new();
    while !parser.is_end() {
        parser.consume_separators();
        if parser.is_end() {
            break;
        }
        let stmt = parser.parse_statement()?;
        statements.push(stmt);
        parser.consume_separators();
    }
    Ok(Program { statements })
}

fn tokenize(code: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            '\n' | ';' => {
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                if i + 1 < chars.len() && chars[i + 1] == '/' {
                    i += 2;
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                } else {
                    tokens.push(Token::Slash);
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Equal);
                i += 1;
            }
            '@' => {
                tokens.push(Token::At);
                i += 1;
            }
            '.' => {
                if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    let start = i;
                    i += 1;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let number: f32 = chars[start..i]
                        .iter()
                        .collect::<String>()
                        .parse()
                        .map_err(|_| "Invalid number literal".to_string())?;
                    tokens.push(Token::Number(number));
                } else {
                    tokens.push(Token::Dot);
                    i += 1;
                }
            }
            '0'..='9' => {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let number: f32 = chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .map_err(|_| "Invalid number literal".to_string())?;
                tokens.push(Token::Number(number));
            }
            '_' | 'a'..='z' | 'A'..='Z' => {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident = chars[start..i].iter().collect::<String>();
                tokens.push(Token::Ident(ident));
            }
            _ => {
                return Err(format!("Unexpected character '{}'", c));
            }
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn is_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn consume_separators(&mut self) {
        while matches!(self.peek(), Some(Token::Semicolon)) {
            self.pos += 1;
        }
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::At)?;
        let target = match self.next() {
            Some(Token::Ident(name)) => name,
            _ => return Err("Expected attribute name after '@'".to_string()),
        };
        self.expect(Token::Equal)?;
        let expr = self.parse_expr()?;
        Ok(Statement::Assign { target, expr })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_mul_div()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.pos += 1;
                    let right = self.parse_mul_div()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                Some(Token::Minus) => {
                    self.pos += 1;
                    let right = self.parse_mul_div()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Sub,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Mul,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                Some(Token::Slash) => {
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Div,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Plus) => {
                self.pos += 1;
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Pos,
                    expr: Box::new(expr),
                })
            }
            Some(Token::Minus) => {
                self.pos += 1;
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if !matches!(self.peek(), Some(Token::Dot)) {
                break;
            }
            self.pos += 1;
            let mask = match self.next() {
                Some(Token::Ident(name)) => name,
                _ => return Err("Expected swizzle mask after '.'".to_string()),
            };
            expr = Expr::Swizzle {
                expr: Box::new(expr),
                mask,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Number(value)) => Ok(Expr::Literal(Value::Float(value))),
            Some(Token::At) => match self.next() {
                Some(Token::Ident(name)) => Ok(Expr::Attr(name)),
                _ => Err("Expected attribute name after '@'".to_string()),
            },
            Some(Token::Ident(name)) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expr()?);
                            match self.peek() {
                                Some(Token::Comma) => {
                                    self.pos += 1;
                                }
                                Some(Token::RParen) => break,
                                _ => return Err("Expected ',' or ')' in function call".to_string()),
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if name == "PI" {
                    Ok(Expr::Literal(Value::Float(std::f32::consts::PI)))
                } else if name == "E" {
                    Ok(Expr::Literal(Value::Float(std::f32::consts::E)))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            other => Err(format!("Unexpected token {:?}", other)),
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), String> {
        match self.next() {
            Some(t) if t == token => Ok(()),
            other => Err(format!("Expected {:?}, got {:?}", token, other)),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::AttributeStorage;
    use crate::volume::VolumeKind;

    #[test]
    fn wrangle_ptnum_sets_point_attribute() {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let mut mesh = Mesh::with_positions_indices(positions, Vec::new());
        apply_wrangle(
            &mut mesh,
            AttributeDomain::Point,
            "@id = @ptnum;",
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let Some(AttributeStorage::Float(values)) =
            mesh.attributes.get(AttributeDomain::Point, "id")
        else {
            panic!("Expected float attribute 'id' on points");
        };
        assert_eq!(values, &vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn wrangle_point_query_secondary_mesh() {
        let mut mesh = Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], Vec::new());
        let secondary = Mesh::with_positions_indices(
            vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
            Vec::new(),
        );

        apply_wrangle(
            &mut mesh,
            AttributeDomain::Point,
            "@P = point(1, P, @ptnum);",
            None,
            Some(&secondary),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(mesh.positions, secondary.positions);
    }

    #[test]
    fn wrangle_point_query_secondary_splats() {
        let mut splats = SplatGeo::with_len(2);
        splats.positions = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let mut secondary = SplatGeo::with_len(2);
        secondary.positions = vec![[3.0, 0.0, 0.0], [6.0, 0.0, 0.0]];

        apply_wrangle_splats(
            &mut splats,
            AttributeDomain::Point,
            "@P = splat(1, P, @ptnum);",
            None,
            Some(&secondary),
            None,
            None,
        )
        .unwrap();

        assert_eq!(splats.positions, secondary.positions);
    }

    #[test]
    fn wrangle_splat_query_secondary_from_mesh() {
        let mut mesh =
            Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], Vec::new());
        let mut secondary = SplatGeo::with_len(2);
        secondary.positions = vec![[5.0, 0.0, 0.0], [7.0, 0.0, 0.0]];

        apply_wrangle(
            &mut mesh,
            AttributeDomain::Point,
            "@P = splat(1, P, @ptnum);",
            None,
            None,
            None,
            Some(&secondary),
            None,
            None,
        )
        .unwrap();

        assert_eq!(mesh.positions, secondary.positions);
    }

    #[test]
    fn wrangle_sample_secondary_volume() {
        let mut mesh = Mesh::with_positions_indices(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            Vec::new(),
        );
        let volume = Volume::new(
            VolumeKind::Density,
            [0.0, 0.0, 0.0],
            [2, 1, 1],
            1.0,
            vec![0.1, 0.9],
        );

        apply_wrangle(
            &mut mesh,
            AttributeDomain::Point,
            "@val = sample(1, @P);",
            None,
            None,
            None,
            None,
            None,
            Some(&volume),
        )
        .unwrap();

        let Some(AttributeStorage::Float(values)) =
            mesh.attributes.get(AttributeDomain::Point, "val")
        else {
            panic!("Expected float attribute 'val' on points");
        };
        assert_eq!(values.len(), 2);
        assert!((values[0] - 0.1).abs() < 1.0e-4);
        assert!((values[1] - 0.9).abs() < 1.0e-4);
    }
}
