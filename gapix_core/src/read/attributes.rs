use std::{
    collections::{hash_map::Entry, HashMap},
    str::FromStr,
};

use anyhow::{bail, Result};
use quick_xml::{events::BytesStart, Reader};

use super::XmlReaderConversions;

#[derive(Debug)]
pub(crate) struct Attributes {
    data: HashMap<String, String>,
}

impl Attributes {
    /// Creates a new Attributes object by parsing out all the attributes of the
    /// specified tag.
    pub(crate) fn new<R>(tag: &BytesStart<'_>, xml_reader: &Reader<R>) -> Result<Self> {
        let mut data = HashMap::new();

        for attr in tag.attributes() {
            let attr = attr?;
            let key = attr.key.into_inner();
            let key = xml_reader.bytes_to_string(key)?;
            let value = xml_reader.cow_to_string(attr.value)?;

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
        let value = match self.data.entry(key.into()) {
            Entry::Occupied(occupied_entry) => occupied_entry.remove(),
            _ => bail!("Mandatory attribute 'id' was missing on the 'email' element"),
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
