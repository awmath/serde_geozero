//! A collector for geometry and property data from geospatial sources.
//!
//! This module provides the [`GeozeroCollector`] which implements the necessary geozero traits
//! to collect geometries and properties from various geospatial data sources.
//!
//! # Example
//!
//! Reading features from `GeoJSON`:
//!
//! ```rust
//! use std::collections::HashMap;
//! use geozero::{GeozeroDatasource};
//! use geo::Geometry;
//! use serde_geozero::collector::GeozeroCollector;
//!
//! // Sample GeoJSON data
//! let geojson = r#"{
//!     "type": "Feature",
//!     "geometry": {
//!         "type": "Point",
//!         "coordinates": [102.0, 0.5]
//!     },
//!     "properties": {
//!         "name": "Test Point",
//!         "value": 42
//!     }
//! }"#;
//!
//! // Initialize collector
//! let mut collector = GeozeroCollector::new();
//!
//! // Process GeoJSON
//! let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
//! reader.process(&mut collector).unwrap();
//!
//! // Access collected features
//! let feature = &collector.features[0];
//! if let Geometry::Point(point) = &feature.geometry {
//!     println!("Point coordinates: ({}, {})", point.x(), point.y());
//! }
//!
//! // Access properties
//! let name = feature.properties.get("name").unwrap().as_str().unwrap();
//! let value = feature.properties.get("value").unwrap().as_i64().unwrap();
//!
//! ```
//!
//! Reading features from ``FlatGeoBuf``
//!
//! ```rust
//! use flatgeobuf:: FgbReader;
//! use geo::Geometry;
//! use geozero::{GeozeroDatasource, ToWkt};
//! use std::io::BufReader;
//! use std::fs::File;
//! use serde_geozero::collector::GeozeroCollector;
//!
//!
//! let mut filein = BufReader::new(File::open("test-data/countries.fgb").unwrap());
//! let mut fgb = FgbReader::open(&mut filein).unwrap().select_all().unwrap();
//!
//! let mut collector = GeozeroCollector::new();
//! fgb.process(&mut collector);
//!
//! let feature = &collector.features[0];
//! println!("Polygon WKT: {}", feature.geometry.to_wkt().unwrap());
//!
//! let name  = feature.properties.get("name").unwrap().as_str().unwrap();
//! ```
#![allow(clippy::many_single_char_names)]
use crate::{error::Error, ser::ColumnValueSerializer};
use std::{collections::HashMap, marker::PhantomData};

use geo::Geometry;
use geozero::{
    error::GeozeroError, geo_types::GeoWriter, ColumnValue, FeatureProcessor, GeomProcessor,
    PropertyProcessor,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeozeroFeature {
    pub geometry: Geometry,
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
}

impl<'de> serde::de::Deserializer<'de> for GeozeroFeature {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        // Convert GeozeroFeature to Value first
        let value = serde_json::to_value(&self).map_err(Error::SerdeError)?;

        // Then use Value's deserializer
        Ok(value.deserialize_any(visitor)?)
    }

    // Forward all other methods to Value's deserializer
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
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
        let geozero_feature = GeozeroFeature {
            geometry: self
                .current_geometry
                .take_geometry()
                .expect("No geometry found."),
            properties: std::mem::take(&mut self.current_properties),
        };
        self.features.push(T::deserialize(geozero_feature)?);
        Ok(())
    }

    fn geometry_begin(&mut self) -> geozero::error::Result<()> {
        self.current_geometry = GeoWriter::new();
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use approx::assert_relative_eq;
    use geo::Geometry;
    use geozero::GeozeroDatasource;

    use crate::collector::GeozeroCollector;

    #[test]
    fn test_from_geojson() -> geozero::error::Result<()> {
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
                        "value": 42
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
                        "value": 43
                    }
                }
            ]
        }"#;

        let mut collector = GeozeroCollector::new();

        let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
        reader.process(&mut collector)?;

        assert_eq!(collector.features.len(), 2);

        // Check first feature
        let feature = &collector.features[0];
        match &feature.geometry {
            Geometry::Point(point) => {
                assert_relative_eq!(point.x(), 102.0);
                assert_relative_eq!(point.y(), 0.5);
            }
            _ => panic!("Expected Point geometry"),
        }
        assert_eq!(
            feature.properties.get("name").unwrap().as_str().unwrap(),
            "Test Point"
        );
        assert_eq!(
            feature.properties.get("value").unwrap().as_i64().unwrap(),
            42
        );

        // Check second feature
        let feature = &collector.features[1];
        match &feature.geometry {
            Geometry::Point(point) => {
                assert_relative_eq!(point.x(), 103.0);
                assert_relative_eq!(point.y(), 1.5);
            }
            _ => panic!("Expected Point geometry"),
        }
        assert_eq!(
            feature.properties.get("name").unwrap().as_str().unwrap(),
            "Another Point"
        );
        assert_eq!(
            feature.properties.get("value").unwrap().as_i64().unwrap(),
            43
        );

        Ok(())
    }
}
