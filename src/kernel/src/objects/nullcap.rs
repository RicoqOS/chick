use crate::objects::capability::CapRef;

#[derive(Debug)]
pub enum NullObj {}

pub type NullCap<'a> = CapRef<'a, NullObj>;
