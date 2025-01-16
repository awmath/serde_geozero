use geozero::{
    error::GeozeroError, geo_types::process_geom, ColumnValue, FeatureProcessor, PropertyProcessor,
};
use hashbrown::HashMap;
use serde::{ser, Deserialize};

use crate::{
    de::Feature,
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
/// # Panics
/// If unsupported fields arise.
/// # Errors
///
pub fn process_properties<P: PropertyProcessor, S: ::std::hash::BuildHasher>(
    properties: &HashMap<String, JsonValue, S>,
    column_mapping: &mut HashMap<String, usize, S>,
    processor: &mut P,
) -> Result<()> {
    for (key, value) in properties {
        let id = if let Some(val) = column_mapping.get(key) {
            *val
        } else {
            let new_id = column_mapping.len();
            column_mapping.insert(key.clone(), column_mapping.len());
            new_id
        };
        match value {
            JsonValue::String(v) => processor.property(id, key, &ColumnValue::String(v))?,
            JsonValue::Number(v) => {
                if v.is_f64() {
                    processor.property(id, key, &ColumnValue::Double(v.as_f64().unwrap()))?
                } else if v.is_i64() {
                    processor.property(id, key, &ColumnValue::Long(v.as_i64().unwrap()))?
                } else if v.is_u64() {
                    processor.property(id, key, &ColumnValue::ULong(v.as_u64().unwrap()))?
                } else {
                    unreachable!()
                }
            }
            JsonValue::Bool(v) => processor.property(id, key, &ColumnValue::Bool(*v))?,
            JsonValue::Array(v) => {
                let json_string =
                    serde_json::to_string(v).map_err(|_err| GeozeroError::Property(key.clone()))?;
                processor.property(id, key, &ColumnValue::Json(&json_string))?
            }
            JsonValue::Object(v) => {
                let json_string =
                    serde_json::to_string(v).map_err(|_err| GeozeroError::Property(key.clone()))?;
                processor.property(id, key, &ColumnValue::Json(&json_string))?
            }
            // For null values omit the property
            JsonValue::Null => false,
        };
    }
    Ok(())
}

/// Converts a slice of serializable features into a `GeoZero` data source.
///
/// This function processes a collection of features and writes them to a `GeoZero` processor.
/// It handles both geometry and property data for each feature.
///
/// # Arguments
///
/// * `input` - A slice of features that implement `ser::Serialize`
/// * `processor` - A mutable reference to a `GeoZero` feature processor
///
/// # Examples
///
/// ```
/// use geo::point;
/// use geozero::geojson::GeoJsonWriter;
/// use hashbrown::HashMap;
/// use serde_geozero::de::Feature;
/// use serde_geozero::to_geozero_datasource;
///
/// // Create sample features
/// let feature = Feature::new(
///     (point! { x: 123.4, y: 345.6 }).into(),
///     HashMap::from_iter(vec![
///         ("name".to_string(), serde_json::to_value("Location A").unwrap()),
///         ("value".to_string(), serde_json::to_value(42).unwrap()),
///     ]),
/// );
///
/// // Prepare GeoJSON writer
/// let mut output = Vec::new();
/// let mut writer = GeoJsonWriter::new(&mut output);
///
/// // Process features
/// to_geozero_datasource(&[feature], &mut writer).unwrap();
///
/// // Result will be GeoJSON data in the output buffer
/// ```
///
/// # Errors
///
/// Returns an error if:
/// * Serialization of input features fails
/// * Processing of geometry or properties fails
/// * Any `GeoZero` processing operation fails
pub fn to_geozero_datasource<T: ser::Serialize, S: FeatureProcessor>(
    input: &[T],
    processor: &mut S,
) -> Result<()> {
    processor.dataset_begin(None)?;
    let mut columns: hashbrown::HashMap<String, usize> = HashMap::new();
    for (fid, data) in input.iter().enumerate() {
        processor.feature_begin(fid as u64)?;
        let deserialized = serde_json::to_value(data)
            .and_then(Feature::deserialize)
            .map_err(Error::SerdeError)?;
        process_geom(&deserialized.geometry, processor)?;

        processor.properties_begin()?;
        process_properties(&deserialized.properties, &mut columns, processor)?;
        processor.properties_end()?;
        processor.feature_end(fid as u64)?;
    }
    processor.dataset_end()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::str::from_utf8;

    use geo::point;
    use geozero::geojson::GeoJsonWriter;
    use hashbrown::HashMap;

    use crate::de::Feature;

    use super::to_geozero_datasource;

    #[test]
    fn test_to_geojson() {
        let data_1 = Feature::new(
            (point! { x: 123.4, y: 345.6 }).into(),
            HashMap::from_iter(vec![
                ("prop1".to_string(), serde_json::to_value(1.).unwrap()),
                ("prop2".to_string(), serde_json::to_value("123").unwrap()),
            ]),
        );
        let data_2 = Feature::new(
            (point! { x: 123.4, y: 345.6 }).into(),
            HashMap::from_iter(vec![
                ("prop1".to_string(), serde_json::to_value(1.).unwrap()),
                ("prop2".to_string(), serde_json::to_value("1234").unwrap()),
            ]),
        );

        let mut out = Vec::new();

        let mut writer = GeoJsonWriter::new(&mut out);
        let data_vec = vec![data_1, data_2];

        assert!(to_geozero_datasource(data_vec.as_slice(), &mut writer).is_ok());

        let string = from_utf8(out.as_slice()).unwrap();

        assert!(string.contains("\"type\": \"Feature\""));
        assert!(string.contains("\"coordinates\": [123.4,345.6]"));
        assert!(string.contains("\"prop1\": 1"));
        assert!(string.contains("\"prop2\": \"123\""));
        assert!(string.contains("\"prop2\": \"1234\""));
    }
}
