use geozero::ColumnValue;
use serde::ser;


pub struct ColumnValueSerializer<'a>(pub &'a ColumnValue<'a>);

impl ser::Serialize for ColumnValueSerializer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match &self.0 {
            ColumnValue::Byte(val) => serializer.serialize_i8(*val),
            ColumnValue::UByte(val) => serializer.serialize_u8(*val),
            ColumnValue::Bool(val) => serializer.serialize_bool(*val),
            ColumnValue::Short(val) => serializer.serialize_i16(*val),
            ColumnValue::UShort(val) => serializer.serialize_u16(*val),
            ColumnValue::Int(val) => serializer.serialize_i32(*val),
            ColumnValue::UInt(val) => serializer.serialize_u32(*val),
            ColumnValue::Long(val) => serializer.serialize_i64(*val),
            ColumnValue::ULong(val) => serializer.serialize_u64(*val),
            ColumnValue::Float(val) => serializer.serialize_f32(*val),
            ColumnValue::Double(val) => serializer.serialize_f64(*val),
            ColumnValue::String(val) | ColumnValue::Json(val) => serializer.serialize_str(val),
            ColumnValue::DateTime(val) => serializer.serialize_str(val.as_ref()),
            ColumnValue::Binary(val) => serializer.serialize_bytes(val),
        }
    }
}
