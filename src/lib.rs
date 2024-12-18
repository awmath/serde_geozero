//! # serde-geozero
//!
//! A library for serializing and deserializing geospatial data using serde and geozero.
//!
//! This crate provides functionality to convert between geospatial data sources and
//! Rust types using serde's serialization framework and geozero's processing capabilities.
//!
//! ## Disclaimer ##
//! This isn't a fully fledged cargo crate as it's still missing some functionality it claims to
//! provide (serialization).
//!
//! ## Features
//!
//! - Deserialize from various geospatial formats (`GeoJSON`, `FlatGeobuf`, etc.) into Rust structs
//! - Collect geometry and property data from geospatial sources
//! - Serialize geometry data into different formats
//!
//! ## Main Components
//!
//! - [`collector::GeozeroCollector`] - Collects geometry and property data from geospatial sources
//! - [`from_datasource`] - Helper function to deserialize data from any `GeozeroDatasource`
//! - [`error::Error`] - Custom error types for the library
//!
//! ## Example
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use geo::Geometry;
//! use serde_geozero::from_datasource;
//! use std::str::from_utf8;
//! use serde_geozero::ser::to_geozero_datasource;
//! use geozero::geojson::GeoJsonWriter;
//!
//! #[derive(Deserialize, Serialize)]
//! struct City {
//!     geometry: Geometry,
//!     name: String,
//!     population: i64,
//! }
//!
//! let geojson = r#"{
//!     "type": "Feature",
//!     "geometry": {
//!         "type": "Point",
//!         "coordinates": [13.4, 52.5]
//!     },
//!     "properties": {
//!         "name": "Berlin",
//!         "population": 3669495
//!     }
//! }"#;
//!
//! let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
//! let cities: Vec<City> = from_datasource(&mut reader).unwrap();
//!
//!
//! let mut out = Vec::new();
//!
//! let mut writer = GeoJsonWriter::new(&mut out);
//!
//! assert!(to_geozero_datasource(cities.as_slice(), &mut writer).is_ok());
//! assert_eq!(
//!     from_utf8(out.as_slice()).unwrap().to_string().retain(|c| !c.is_whitespace()),
//!     geojson.to_string().retain(|c| !c.is_whitespace())
//!     );
//! ```
//!
//! ## TODO:
//!  - Serialization
//!  - Deserialization for non `GeozeroDatasource`
//!
//! ## Modules
//!
//! - [`collector`] - Contains the `GeozeroCollector` implementation
//! - [`de`] - Deserialization functionality
//! - [`error`] - Error types and handling
//! - [`ser`] - Serialization functionality

#[allow(clippy::module_name_repetitions)]
pub mod de;
pub mod error;
pub mod ser;

pub use de::from_datasource;
