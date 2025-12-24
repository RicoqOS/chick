use crate::objects::capacity::ObjType;
use crate::objects::nullcap::NullObj;

pub trait KernelObject {
    const OBJ_TYPE: ObjType;
}

impl KernelObject for NullObj {
    const OBJ_TYPE: ObjType = ObjType::NullObj;
}
