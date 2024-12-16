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
//! - Deserialize from various geospatial formats (GeoJSON, FlatGeobuf, etc.) into Rust structs
//! - Collect geometry and property data from geospatial sources
//! - Serialize geometry data into different formats
//!
//! ## Main Components
//!
//! - [`collector::GeozeroCollector`] - Collects geometry and property data from geospatial sources
//! - [`from_datasource`] - Helper function to deserialize data from any GeozeroDatasource
//! - [`error::Error`] - Custom error types for the library
//!
//! ## Example
//!
//! ```rust
//! use serde::Deserialize;
//! use geo::Geometry;
//! use serde_geozero::from_datasource;
//!
//! #[derive(Deserialize)]
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
//! ```
//!
//! ## TODO:
//!  - Serialization
//!  - Deserialization for non GeozeroDatasource
//!
//! ## Modules
//!
//! - [`collector`] - Contains the GeozeroCollector implementation
//! - [`de`] - Deserialization functionality
//! - [`error`] - Error types and handling
//! - [`ser`] - Serialization functionality
pub mod collector;
pub mod de;
pub mod error;
pub mod ser;

pub use de::from_datasource;
