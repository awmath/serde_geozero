use geozero::GeozeroDatasource;
use serde::Deserialize;

use crate::{
    collector::GeozeroCollector,
    error::{Error, Result},
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

    collector
        .features
        .into_iter()
        .map(|feature| {
            serde_json::to_value(feature)
                .and_then(T::deserialize)
                .map_err(Error::SerdeError)
        })
        .collect()
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
                        "name": "Test Point"
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [103.0, 1.5]
                    },
                    "properties": {
                        "name": "Another Point"
                    }
                }
            ]
        }"#;

        let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
        let features: Vec<Test> = from_datasource(&mut reader)?;

        assert_eq!(features.len(), 2);

        // Check first feature
        assert_eq!(features[0].title, "Test Point");
        match &features[0].geometry {
            Geometry::Point(point) => {
                assert_relative_eq!(point.x(), 102.0);
                assert_relative_eq!(point.y(), 0.5);
            }
            _ => panic!("Expected Point geometry"),
        }

        // Check second feature
        assert_eq!(features[1].title, "Another Point");
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
