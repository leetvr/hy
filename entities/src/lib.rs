/// This crate contains data types relevant to entities
///
/// It is designed to be used on both the server and the client

use tsify::Tsify;

#[derive(Tsify)]
pub struct EntityType {
    /// Name appropriate for display to the user
    pub name: String,
    /// The properties custom to this EntityType (i.e. not including "standard" properties)
    pub custom_properties: Vec<EntityPropertyType>
}

#[derive(Tsify)]
pub struct EntityPropertyType {
    /// Name appropriate for display to the user
    pub name: String,
    /// What are the valid values for this property?
    pub property_kind: PropertyKind,
}

#[derive(Tsify)]
pub enum PropertyKind {
    Float,
    Int,
    Options(Vec<String>),
}
