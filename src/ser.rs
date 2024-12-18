use std::collections::HashMap;

use geozero::{
    error::GeozeroError, geo_types::process_geom, ColumnValue, FeatureProcessor, PropertyProcessor,
};
use serde::{ser, Deserialize};

use crate::{
    de::GeozeroFeature,
    error::{Error, Result},
};
use serde_json::Value as JsonValue;

pub struct ColumnValueSerializer<'a>(pub &'a ColumnValue<'a>);

impl ser::Serialize for ColumnValueSerializer<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
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

/// borrowed from geozero as it is private
fn process_properties<P: PropertyProcessor>(
    properties: &HashMap<String, JsonValue>,
    processor: &mut P,
) -> Result<()> {
    for (i, (key, value)) in properties.iter().enumerate() {
        // Could we provide a stable property index?
        match value {
            JsonValue::String(v) => processor.property(i, key, &ColumnValue::String(v))?,
            JsonValue::Number(v) => {
                if v.is_f64() {
                    processor.property(i, key, &ColumnValue::Double(v.as_f64().unwrap()))?
                } else if v.is_i64() {
                    processor.property(i, key, &ColumnValue::Long(v.as_i64().unwrap()))?
                } else if v.is_u64() {
                    processor.property(i, key, &ColumnValue::ULong(v.as_u64().unwrap()))?
                } else {
                    unreachable!()
                }
            }
            JsonValue::Bool(v) => processor.property(i, key, &ColumnValue::Bool(*v))?,
            JsonValue::Array(v) => {
                let json_string =
                    serde_json::to_string(v).map_err(|_err| GeozeroError::Property(key.clone()))?;
                processor.property(i, key, &ColumnValue::Json(&json_string))?
            }
            JsonValue::Object(v) => {
                let json_string =
                    serde_json::to_string(v).map_err(|_err| GeozeroError::Property(key.clone()))?;
                processor.property(i, key, &ColumnValue::Json(&json_string))?
            }
            // For null values omit the property
            JsonValue::Null => false,
        };
    }
    Ok(())
}
///
/// # Errors
pub fn to_geozero_datasource<T: ser::Serialize, S: FeatureProcessor>(
    input: &[T],
    processor: &mut S,
) -> Result<()> {
    processor.dataset_begin(None)?;
    for (fid, data) in input.iter().enumerate() {
        processor.feature_begin(fid as u64)?;
        let deserialized = serde_json::to_value(data)
            .and_then(GeozeroFeature::deserialize)
            .map_err(Error::SerdeError)?;
        process_geom(&deserialized.geometry, processor)?;

        processor.properties_begin()?;
        process_properties(&deserialized.properties, processor)?;
        processor.properties_end()?;
        processor.feature_end(fid as u64)?;
    }
    processor.dataset_end()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, str::from_utf8};

    use geo::point;
    use geozero::geojson::GeoJsonWriter;

    use crate::de::GeozeroFeature;

    use super::to_geozero_datasource;

    #[test]
    fn test_to_geojson() {
        let data = GeozeroFeature::new(
            (point! { x: 123.4, y: 345.6 }).into(),
            HashMap::from_iter(vec![
                ("prop1".to_string(), serde_json::to_value(1.).unwrap()),
                ("prop2".to_string(), serde_json::to_value("123").unwrap()),
            ]),
        );

        let mut out = Vec::new();

        let mut writer = GeoJsonWriter::new(&mut out);
        let data_vec = vec![data];

        assert!(to_geozero_datasource(data_vec.as_slice(), &mut writer).is_ok());

        let string = from_utf8(out.as_slice()).unwrap();

        assert!(string.contains("\"type\": \"Feature\""));
        assert!(string.contains("\"coordinates\": [123.4,345.6]"));
        assert!(string.contains("\"prop1\": 1"));
        assert!(string.contains("\"prop2\": \"123\""));
    }
}
