use crate::attributes::AttributeType;

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

impl Value {
    pub fn data_type(self) -> AttributeType {
        match self {
            Value::Float(_) => AttributeType::Float,
            Value::Vec2(_) => AttributeType::Vec2,
            Value::Vec3(_) => AttributeType::Vec3,
            Value::Vec4(_) => AttributeType::Vec4,
        }
    }

    pub fn negate(self) -> Value {
        match self {
            Value::Float(v) => Value::Float(-v),
            Value::Vec2(v) => Value::Vec2([-v[0], -v[1]]),
            Value::Vec3(v) => Value::Vec3([-v[0], -v[1], -v[2]]),
            Value::Vec4(v) => Value::Vec4([-v[0], -v[1], -v[2], -v[3]]),
        }
    }
}
