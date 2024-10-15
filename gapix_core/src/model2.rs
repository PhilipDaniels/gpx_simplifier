use std::path::PathBuf;

use crate::model::{
    Copyright, Email, Gpx, GpxFile, Lat, Link, Lon, Metadata, Waypoint, XmlDeclaration,
};

impl GpxFile {
    /// Creates a new GpxFile value from the mandatory fields.
    pub fn new(declaration: XmlDeclaration, gpx: Gpx, metadata: Metadata) -> Self {
        Self {
            filename: None,
            declaration,
            gpx,
            metadata,
            waypoints: Vec::new(),
            routes: Vec::new(),
            tracks: Vec::new(),
        }
    }

    /// Creates a new GpxFile value from the mandatory fields and the filename.
    pub fn with_filename<P>(
        declaration: XmlDeclaration,
        gpx: Gpx,
        metadata: Metadata,
        filename: P,
    ) -> Self
    where
        P: Into<PathBuf>,
    {
        let mut v = Self::new(declaration, gpx, metadata);
        v.filename = Some(filename.into());
        v
    }
}

impl Default for XmlDeclaration {
    fn default() -> Self {
        Self {
            version: "1.0".to_owned(),
            encoding: Some("UTF-8".to_owned()),
            standalone: Default::default(),
        }
    }
}

impl Default for Gpx {
    /// Creates a new Gpx value with 'gapix' as the creator.
    fn default() -> Self {
        Self::with_creator("gapix")
    }
}

impl Gpx {
    /// Creates a new Gpx value with the specified creator.
    pub fn with_creator<S>(creator: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            version: "1.1".to_owned(),
            creator: creator.into(),
            attributes: Default::default(),
        }
    }
}

impl Waypoint {
    pub fn with_lat_lon(lat: Lat, lon: Lon) -> Self {
        let mut v = Self::default();
        v.lat = lat;
        v.lon = lon;
        v
    }
}

impl Copyright {
    /// Constructs a new Copyright value from the two mandatory fields, year and
    /// author.
    pub fn new<S>(year: i16, author: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            year,
            license: None,
            author: author.into(),
        }
    }

    /// Constructs a new Copyright value from the two mandatory fields (year and
    /// author) and the licence.
    pub fn with_licence<S1, S2>(year: i16, author: S1, licence: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            year,
            license: Some(licence.into()),
            author: author.into(),
        }
    }
}

impl Email {
    /// Constructs a new email element from the two mandatory fields, id and
    /// domain.
    pub fn new<S1, S2>(id: S1, domain: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            id: id.into(),
            domain: domain.into(),
        }
    }
}

impl Link {
    pub fn new<S>(href: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            text: None,
            r#type: None,
            href: href.into(),
        }
    }
}
