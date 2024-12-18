#![allow(clippy::many_single_char_names)]
use std::{collections::HashMap, marker::PhantomData};

use geo::Geometry;
use geozero::{
    error::GeozeroError, geo_types::GeoWriter, ColumnValue, FeatureProcessor, GeomProcessor,
    GeozeroDatasource, PropertyProcessor,
};
use serde::{
    de::{value::StringDeserializer, MapAccess},
    Deserialize, Serialize,
};
use serde_json::Value;

use crate::{
    error::{Error, Result},
    ser::ColumnValueSerializer,
};

/// Deserializes data from a `GeozeroDatasource` into a type that implements Deserialize.
///
/// This function takes any `GeozeroDatasource` (like `GeoJSON`, `FlatGeobuf`, etc.) and converts
/// its features into your custom types that implement Deserialize.
///
/// # Examples
///
/// Reading `GeoJSON` into custom structs:
/// ```
/// use serde::Deserialize;
/// use geo::Geometry;
/// use serde_geozero::from_datasource;
///
/// #[derive(Deserialize)]
/// struct City {
///     geometry: Geometry,
///     name: String,
///     population: i64,
/// }
///
/// let geojson = r#"{
///     "type": "Feature",
///     "geometry": {
///         "type": "Point",
///         "coordinates": [13.4, 52.5]
///     },
///     "properties": {
///         "name": "Berlin",
///         "population": 3669495
///     }
/// }"#;
///
/// let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
/// let cities: Vec<City> = from_datasource(&mut reader).unwrap();
///
/// assert_eq!(cities.first().unwrap().name, "Berlin");
/// ```
///
/// Reading `FlatGeobuf` features:
/// ```
/// use serde::Deserialize;
/// use geo::Geometry;
/// use std::fs::File;
/// use flatgeobuf::FgbReader;
/// use serde_geozero::from_datasource;
///
/// #[derive(Deserialize)]
/// struct Country {
///     geometry: Geometry,
///     name: String,
///     id: String,
/// }
///
/// let f = File::open("test-data/countries.fgb").unwrap();
/// let mut reader = FgbReader::open(f).unwrap();
/// let countries: Vec<Country> = from_datasource(&mut reader.select_all().unwrap()).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The datasource processing fails
/// - The collected features cannot be serialized to JSON
/// - The JSON cannot be deserialized into the target type
///
pub fn from_datasource<'de, T: Deserialize<'de>, S: GeozeroDatasource>(
    processor: &mut S,
) -> Result<Vec<T>> {
    let mut collector = GeozeroCollector::new();
    processor.process(&mut collector)?;

    Ok(collector.features)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeozeroFeature {
    pub geometry: Geometry,
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,

    #[serde(skip)]
    map_keys: Vec<String>,

    #[serde(skip)]
    current_col: Option<String>,
}

impl GeozeroFeature {
    #[must_use]
    pub fn new(geometry: Geometry, properties: HashMap<String, Value>) -> Self {
        let map_keys = properties.keys().cloned().collect();

        Self {
            geometry,
            properties,
            current_col: None,
            map_keys,
        }
    }
}

impl<'de> serde::de::Deserializer<'de> for GeozeroFeature {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    // Forward all other methods to Value's deserializer
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

const GEOMETRY_COL: &str = "geometry";

impl<'de> MapAccess<'de> for GeozeroFeature {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        // First return geometry field
        if self.current_col.is_none() {
            self.current_col = Some(GEOMETRY_COL.to_string());
        } else {
            self.current_col = self.map_keys.pop();
        }

        if let Some(col) = &self.current_col {
            return seed
                .deserialize(StringDeserializer::new(col.clone()))
                .map(Some);
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        if self.current_col == Some(GEOMETRY_COL.to_string()) {
            // Return geometry value
            return seed
                .deserialize(serde_json::to_value(&self.geometry).map_err(Error::SerdeError)?)
                .map_err(Error::SerdeError);
        }

        if let Some(col) = &self.current_col {
            if let Some(value) = self.properties.get(col) {
                return seed.deserialize(value.clone()).map_err(Error::SerdeError);
            }
        }

        Err(Error::SerdeError(serde::de::Error::custom(
            "no value found",
        )))
    }
}

pub struct GeozeroCollector<'de, T: Deserialize<'de>> {
    pub features: Vec<T>,

    current_geometry: GeoWriter,
    current_properties: HashMap<String, Value>,
    _phantom: &'de PhantomData<()>,
}

impl<'de, T: Deserialize<'de>> GeozeroCollector<'de, T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
            current_geometry: GeoWriter::new(),
            current_properties: HashMap::new(),
            _phantom: &PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> Default for GeozeroCollector<'de, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de, T: Deserialize<'de>> PropertyProcessor for GeozeroCollector<'de, T> {
    fn property(
        &mut self,
        _idx: usize,
        name: &str,
        value: &ColumnValue,
    ) -> geozero::error::Result<bool> {
        self.current_properties.insert(
            name.to_string(),
            serde_json::to_value(ColumnValueSerializer(value))
                .map_err(|err| GeozeroError::Property(err.to_string()))?,
        );
        Ok(false)
    }
}

impl<'de, T: Deserialize<'de>> GeomProcessor for GeozeroCollector<'de, T> {
    fn dimensions(&self) -> geozero::CoordDimensions {
        self.current_geometry.dimensions()
    }

    fn multi_dim(&self) -> bool {
        self.current_geometry.multi_dim()
    }

    fn srid(&mut self, srid: Option<i32>) -> geozero::error::Result<()> {
        self.current_geometry.srid(srid)
    }

    fn xy(&mut self, x: f64, y: f64, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.xy(x, y, idx)
    }

    fn coordinate(
        &mut self,
        x: f64,
        y: f64,
        z: Option<f64>,
        m: Option<f64>,
        t: Option<f64>,
        tm: Option<u64>,
        idx: usize,
    ) -> geozero::error::Result<()> {
        self.current_geometry.coordinate(x, y, z, m, t, tm, idx)
    }

    fn empty_point(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.empty_point(idx)
    }

    fn point_begin(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.point_begin(idx)
    }

    fn point_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.point_end(idx)
    }

    fn multipoint_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multipoint_begin(size, idx)
    }

    fn multipoint_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multipoint_end(idx)
    }

    fn linestring_begin(
        &mut self,
        tagged: bool,
        size: usize,
        idx: usize,
    ) -> geozero::error::Result<()> {
        self.current_geometry.linestring_begin(tagged, size, idx)
    }

    fn linestring_end(&mut self, tagged: bool, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.linestring_end(tagged, idx)
    }

    fn multilinestring_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multilinestring_begin(size, idx)
    }

    fn multilinestring_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multilinestring_end(idx)
    }

    fn polygon_begin(
        &mut self,
        tagged: bool,
        size: usize,
        idx: usize,
    ) -> geozero::error::Result<()> {
        self.current_geometry.polygon_begin(tagged, size, idx)
    }

    fn polygon_end(&mut self, tagged: bool, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.polygon_end(tagged, idx)
    }

    fn multipolygon_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multipolygon_begin(size, idx)
    }

    fn multipolygon_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multipolygon_end(idx)
    }

    fn geometrycollection_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.geometrycollection_begin(size, idx)
    }

    fn geometrycollection_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.geometrycollection_end(idx)
    }

    fn circularstring_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.circularstring_begin(size, idx)
    }

    fn circularstring_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.circularstring_end(idx)
    }

    fn compoundcurve_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.compoundcurve_begin(size, idx)
    }

    fn compoundcurve_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.compoundcurve_end(idx)
    }

    fn curvepolygon_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.curvepolygon_begin(size, idx)
    }

    fn curvepolygon_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.curvepolygon_end(idx)
    }

    fn multicurve_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multicurve_begin(size, idx)
    }

    fn multicurve_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multicurve_end(idx)
    }

    fn multisurface_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multisurface_begin(size, idx)
    }

    fn multisurface_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.multisurface_end(idx)
    }

    fn triangle_begin(
        &mut self,
        tagged: bool,
        size: usize,
        idx: usize,
    ) -> geozero::error::Result<()> {
        self.current_geometry.triangle_begin(tagged, size, idx)
    }

    fn triangle_end(&mut self, tagged: bool, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.triangle_end(tagged, idx)
    }

    fn polyhedralsurface_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.polyhedralsurface_begin(size, idx)
    }

    fn polyhedralsurface_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.polyhedralsurface_end(idx)
    }

    fn tin_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.tin_begin(size, idx)
    }

    fn tin_end(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.current_geometry.tin_end(idx)
    }

    fn pre_process_xy<F: Fn(&mut f64, &mut f64)>(
        self,
        transform_xy: F,
    ) -> geozero::WrappedXYProcessor<Self, F>
    where
        Self: Sized,
    {
        geozero::WrappedXYProcessor::new(self, transform_xy)
    }
}

impl<'de, T: Deserialize<'de>> FeatureProcessor for GeozeroCollector<'de, T> {
    fn properties_begin(&mut self) -> geozero::error::Result<()> {
        self.current_properties = HashMap::new();
        Ok(())
    }

    fn feature_end(&mut self, _idx: u64) -> geozero::error::Result<()> {
        let geozero_feature = GeozeroFeature::new(
            self.current_geometry
                .take_geometry()
                .expect("No geometry found."),
            std::mem::take(&mut self.current_properties),
        );
        self.features.push(
            T::deserialize(geozero_feature)
                .map_err(|err| GeozeroError::Feature(err.to_string()))?,
        );
        Ok(())
    }

    fn geometry_begin(&mut self) -> geozero::error::Result<()> {
        self.current_geometry = GeoWriter::new();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use flatgeobuf::FgbReader;
    use geo::Geometry;
    use serde::{Deserialize, Serialize};
    use std::fs::File;

    #[test]
    fn test_flatgeobuf() -> anyhow::Result<()> {
        #[derive(Debug, Deserialize)]
        struct Country {
            geometry: Geometry,
            name: String,
            id: String,
        }

        let f = File::open("test-data/countries.fgb")?;
        let reader = FgbReader::open(f)?;
        let features: Vec<Country> = from_datasource(&mut reader.select_all()?)?;

        assert!(!features.is_empty());

        // Check first feature has expected fields
        let first = &features[0];
        assert_eq!(first.name, "Antarctica");
        assert_eq!(first.id, "ATA");
        assert!(matches!(first.geometry, Geometry::MultiPolygon(_)));

        Ok(())
    }

    #[test]
    fn test_geojson() -> Result<()> {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Test {
            geometry: Geometry,
            #[serde(rename = "name")]
            title: String,
            value: u8,
        }

        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [102.0, 0.5]
                    },
                    "properties": {
                        "name": "Test Point",
                        "value": 1
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [103.0, 1.5]
                    },
                    "properties": {
                        "name": "Another Point",
                        "value": 10
                    }
                }
            ]
        }"#;

        let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
        let features: Vec<Test> = from_datasource(&mut reader)?;

        assert_eq!(features.len(), 2);

        // Check first feature
        assert_eq!(features[0].title, "Test Point");
        assert_eq!(features[0].value, 1);
        match &features[0].geometry {
            Geometry::Point(point) => {
                assert_relative_eq!(point.x(), 102.0);
                assert_relative_eq!(point.y(), 0.5);
            }
            _ => panic!("Expected Point geometry"),
        }

        // Check second feature
        assert_eq!(features[1].title, "Another Point");
        assert_eq!(features[1].value, 10);
        match &features[1].geometry {
            Geometry::Point(point) => {
                assert_relative_eq!(point.x(), 103.0);
                assert_relative_eq!(point.y(), 1.5);
            }
            _ => panic!("Expected Point geometry"),
        }

        Ok(())
    }
}
