use serde::de::{
    Deserializer, IntoDeserializer, Visitor,
    value::{Error as ValueDeError, MapAccessDeserializer},
};
use serde_json::Value;

use crate::path::join_path;

use super::access::{CoercingMapAccess, CoercingSeqAccess};
use super::model::CoercingDeserializer;
use super::scalar;

macro_rules! deserialize_integer_from_value {
    ($method:ident, $visit:ident, $ty:ty) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            scalar::deserialize_integer::<_, $ty, _>(&self, visitor, |visitor, value| {
                visitor.$visit(value)
            })
        }
    };
}

macro_rules! deserialize_float_from_value {
    ($method:ident, $visit:ident, $ty:ty) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            scalar::deserialize_float::<_, $ty, _>(&self, visitor, |visitor, value| {
                visitor.$visit(value)
            })
        }
    };
}

impl<'de, 'a> Deserializer<'de> for CoercingDeserializer<'a>
where
    'a: 'de,
{
    type Error = ValueDeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(value) => visitor.visit_bool(*value),
            Value::Number(number) => {
                if let Some(value) = number.as_i64() {
                    visitor.visit_i64(value)
                } else if let Some(value) = number.as_u64() {
                    visitor.visit_u64(value)
                } else if let Some(value) = number.as_f64() {
                    visitor.visit_f64(value)
                } else {
                    Err(self.invalid_type(&visitor))
                }
            }
            Value::String(value) => visitor.visit_borrowed_str(value),
            Value::Array(values) => visitor.visit_seq(CoercingSeqAccess::new(
                values.iter().enumerate(),
                self.path,
                self.string_coercion_paths,
                self.known_paths,
                self.ignored_paths,
            )),
            Value::Object(map) => visitor.visit_map(CoercingMapAccess::new(
                map.iter(),
                self.path,
                self.string_coercion_paths,
                self.known_paths,
                self.ignored_paths,
            )),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(raw) = self.coercible_string() {
            return match scalar::parse_bool(raw) {
                Some(value) => visitor.visit_bool(value),
                None => Err(self.invalid_string_type(raw, &visitor)),
            };
        }

        match self.value {
            Value::Bool(value) => visitor.visit_bool(*value),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    deserialize_integer_from_value!(deserialize_i8, visit_i8, i8);
    deserialize_integer_from_value!(deserialize_i16, visit_i16, i16);
    deserialize_integer_from_value!(deserialize_i32, visit_i32, i32);
    deserialize_integer_from_value!(deserialize_i64, visit_i64, i64);
    deserialize_integer_from_value!(deserialize_i128, visit_i128, i128);
    deserialize_integer_from_value!(deserialize_u8, visit_u8, u8);
    deserialize_integer_from_value!(deserialize_u16, visit_u16, u16);
    deserialize_integer_from_value!(deserialize_u32, visit_u32, u32);
    deserialize_integer_from_value!(deserialize_u64, visit_u64, u64);
    deserialize_integer_from_value!(deserialize_u128, visit_u128, u128);
    deserialize_float_from_value!(deserialize_f32, visit_f32, f32);
    deserialize_float_from_value!(deserialize_f64, visit_f64, f64);

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let Value::String(value) = self.value else {
            return Err(self.invalid_type(&visitor));
        };
        let mut chars = value.chars();
        match (chars.next(), chars.next()) {
            (Some(ch), None) => visitor.visit_char(ch),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_borrowed_str(value),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_string(value.clone()),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_borrowed_bytes(value.as_bytes()),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_byte_buf(value.as_bytes().to_vec()),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if matches!(self.value, Value::Null) {
            return visitor.visit_none();
        }

        if let Some(raw) = self.coercible_string()
            && raw.trim() == "null"
        {
            return visitor.visit_none();
        }

        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if matches!(self.value, Value::Null) {
            return visitor.visit_unit();
        }

        if let Some(raw) = self.coercible_string()
            && raw.trim() == "null"
        {
            return visitor.visit_unit();
        }

        Err(self.invalid_type(&visitor))
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Array(values) => visitor.visit_seq(CoercingSeqAccess::new(
                values.iter().enumerate(),
                self.path,
                self.string_coercion_paths,
                self.known_paths,
                self.ignored_paths,
            )),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Value::Array(values) = self.value {
            for index in 0..len {
                self.record_known_path(&join_path(&self.path, &index.to_string()));
            }
            for index in len..values.len() {
                self.record_ignored_path(&join_path(&self.path, &index.to_string()));
            }
        }
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Object(map) => visitor.visit_map(CoercingMapAccess::new(
                map.iter(),
                self.path,
                self.string_coercion_paths,
                self.known_paths,
                self.ignored_paths,
            )),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        for field in fields {
            self.record_known_path(&join_path(&self.path, field));
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_enum(value.as_str().into_deserializer()),
            Value::Object(map) => {
                visitor.visit_enum(MapAccessDeserializer::new(CoercingMapAccess::new(
                    map.iter(),
                    self.path,
                    self.string_coercion_paths,
                    self.known_paths,
                    self.ignored_paths,
                )))
            }
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}
