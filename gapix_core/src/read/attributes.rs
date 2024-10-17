use std::{
    collections::{hash_map::Entry, HashMap},
    str::FromStr,
};

use anyhow::{bail, Result};
use quick_xml::events::BytesStart;

use super::XmlReaderConversions;

#[derive(Debug)]
pub(crate) struct Attributes {
    data: HashMap<String, String>,
}

impl Attributes {
    /// Creates a new Attributes object by parsing out all the attributes of the
    /// specified tag.
    pub(crate) fn new<C: XmlReaderConversions>(
        start_element: &BytesStart<'_>,
        converter: &C,
    ) -> Result<Self> {
        let mut data = HashMap::new();

        for attr in start_element.attributes() {
            let attr = attr?;
            let key = attr.key.into_inner();
            let key = converter.bytes_to_string(key)?;
            let value = converter.cow_to_string(attr.value)?;

            data.insert(key, value);
        }

        Ok(Self { data })
    }

    /// Returns the number of attributes.
    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the attribute set is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn into_inner(self) -> HashMap<String, String> {
        self.data
    }

    /// Gets a mandatory attribute. The attribute is removed from the list
    /// of attributes and returned to the caller.
    pub(crate) fn get<S, T>(&mut self, key: S) -> Result<T>
    where
        S: Into<String>,
        T: FromStr,
    {
        let key = key.into();

        let value = match self.data.entry(key.clone()) {
            Entry::Occupied(occupied_entry) => occupied_entry.remove(),
            _ => bail!("Mandatory attribute '{}' not found", key),
        };

        match value.parse::<T>() {
            Ok(v) => Ok(v),
            Err(_) => bail!(
                "Could not parse {value} into {}",
                std::any::type_name::<T>()
            ),
        }
    }
}
