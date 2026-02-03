# serde-geozero

A Rust library for serializing and deserializing geospatial data using serde and geozero.

## Overview

serde-geozero provides functionality to convert between geospatial data sources and Rust types using serde's serialization framework and geozero's processing capabilities. It enables seamless integration of various geospatial formats like GeoJSON and FlatGeobuf with Rust's type system.

## Features

- Deserialize from various geospatial formats (GeoJSON, FlatGeobuf, etc.) into Rust structs
- Serialize Rust structs into geospatial formats
- Support for geometry and property data
- Type-safe conversion between geospatial and Rust types

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
serde-geozero = "0.1.0"
```

## Usage

### Deserializing GeoJSON

```rust
use serde::Deserialize;
use geo::Geometry;
use serde_geozero::from_datasource;

#[derive(Deserialize)]
struct City {
    geometry: Geometry,
    name: String,
    population: i64,
}

let geojson = r#"{
    "type": "Feature",
    "geometry": {
        "type": "Point",
        "coordinates": [13.4, 52.5]
    },
    "properties": {
        "name": "Berlin",
        "population": 3669495
    }
}"#;

let mut reader = geozero::geojson::GeoJsonReader(geojson.as_bytes());
let cities: Vec<City> = from_datasource(&mut reader).unwrap();

assert_eq!(cities[0].name, "Berlin");
```

### Serializing to GeoJSON

```rust
use geo::point;
use geozero::geojson::GeoJsonWriter;
use serde_geozero::to_geozero_datasource;
use hashbrown::HashMap;
use serde_geozero::de::Feature;

// Create a feature
let feature = Feature::new(
    (point! { x: 123.4, y: 345.6 }).into(),
    HashMap::from_iter(vec![
        ("name".to_string(), serde_json::to_value("Location A").unwrap()),
        ("value".to_string(), serde_json::to_value(42).unwrap()),
    ]),
);

// Serialize to GeoJSON
let mut output = Vec::new();
let mut writer = GeoJsonWriter::new(&mut output);
to_geozero_datasource(&[feature], &mut writer).unwrap();
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Notes

For cargo-release to compile on Fedora 43 you will need to install the perl-Time-Piece package.
